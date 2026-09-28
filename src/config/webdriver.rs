use std::{
    collections::VecDeque,
    env,
    sync::Arc,
    time::{Duration, Instant},
};

use thirtyfour::{
    error::{WebDriverError, WebDriverResult},
    CapabilitiesHelper, ChromeCapabilities, ChromiumLikeCapabilities, PageLoadStrategy, WebDriver,
};
use tokio::sync::{Mutex, OwnedSemaphorePermit, RwLock, Semaphore};

use crate::model::error::PoolError;

#[allow(dead_code)]
pub enum AcquireStrategy {
    // 즉시 가용 드라이버 없으면 실
    FailFast,
    // 지정 시간까지 대기
    Wait(Option<Duration>),
}

#[derive(Clone)]
pub struct DriverPoolConfig {
    pub max_sessions: usize,
    pub selenium_endpoints: Vec<String>,
    pub page_load_strategy: PageLoadStrategy,
    pub window_size: (u32, u32),
    pub keepalive_interval: Option<Duration>,
}

impl Default for DriverPoolConfig {
    fn default() -> Self {
        Self {
            max_sessions: 2,
            selenium_endpoints: vec![
                env::var("SCHEDULER_SELENIUM_URL_1").expect("SCHEDULER_SELENIUM_URL_1 not set"),
                env::var("SCHEDULER_SELENIUM_URL_2").expect("SCHEDULER_SELENIUM_URL_2 not set"),
                //                env::var("SCHEDULER_SELENIUM_URL_3").expect("SCHEDULER_SELENIUM_URL_3 not set"),
                //                env::var("SCHEDULER_SELENIUM_URL_4").expect("SCHEDULER_SELENIUM_URL_4 not set"),
                //                env::var("SCHEDULER_SELENIUM_URL_5").expect("SCHEDULER_SELENIUM_URL_5 not set"),
            ],
            page_load_strategy: PageLoadStrategy::Eager,
            window_size: (1920, 1080),
            keepalive_interval: Some(Duration::from_secs(180)),
        }
    }
}

struct Inner {
    idle: VecDeque<WebDriver>,
    total: usize,
}

#[derive(Clone)]
pub struct DriverPool {
    cfg: DriverPoolConfig,
    inner: Arc<Mutex<Inner>>,
    semaphore: Arc<Semaphore>,
    last_health: Arc<RwLock<Instant>>,
}

pub struct DriverHandle {
    driver: Option<WebDriver>,
    pool: DriverPool,
    // Semaphore permit이 drop되면 대기 중인 다른 작업이 꺠울 수 있음.
    // 드라이버 반납이 끝난 뒤에 풀어야 하므로 release 태스크로 넘긴다.
    permit: Option<OwnedSemaphorePermit>,
    broken: bool,
}

impl DriverPool {
    pub fn new(cfg: DriverPoolConfig) -> Self {
        let cfg_max_sessions = cfg.max_sessions;
        let pool = Self {
            cfg,
            inner: Arc::new(Mutex::new(Inner {
                idle: VecDeque::new(),
                total: 0,
            })),
            // 동시 사용 세션 수 상한. 드라이버가 죽어도 permit은 유지되고, acquire에서 재생성한다.
            semaphore: Arc::new(Semaphore::new(cfg_max_sessions)),
            last_health: Arc::new(RwLock::new(Instant::now())),
        };

        // 백그라운드로 미리 생성
        let clone = pool.clone();
        tokio::spawn(async move {
            clone.preallocate_all().await;
        });

        if let Some(interval) = pool.cfg.keepalive_interval {
            let clone = pool.clone();
            tokio::spawn(async move {
                clone.keepalive_loop(interval).await;
            });
        }
        pool
    }

    async fn preallocate_all(&self) {
        info!("preallocating drivers...");
        for i in 0..self.cfg.max_sessions {
            match self.spawn_driver(i).await {
                Ok(drv) => {
                    {
                        let mut guard = self.inner.lock().await;
                        guard.idle.push_back(drv);
                        guard.total += 1;
                        info!("preallocated a driver, total={}", guard.total);
                    }
                }
                Err(e) => {
                    warn!("[DriverPool] Preallocate failed: {e}");
                }
            }
        }
        info!("[DriverPool] Preallocation done.");
    }

    async fn spawn_driver(&self, index: usize) -> WebDriverResult<WebDriver> {
        let endpoints = &self.cfg.selenium_endpoints;

        let mut caps = ChromeCapabilities::new();
        caps.add_arg("--disable-dev-shm-usage")?;
        caps.add_arg("--no-sandbox")?;
        caps.add_arg(&format!(
            "--window-size={},{}",
            self.cfg.window_size.0, self.cfg.window_size.1
        ))?;
        caps.set_page_load_strategy(self.cfg.page_load_strategy.clone())?;

        // index 위치 엔드포인트부터 순서대로 시도한다.
        for offset in 0..endpoints.len() {
            let endpoint = &endpoints[(index + offset) % endpoints.len()];
            info!("endpoint: {}", endpoint);
            match WebDriver::new(endpoint, caps.clone()).await {
                Ok(drv) => {
                    info!("[DriverPool] New session created at {}", endpoint);
                    return Ok(drv);
                }
                Err(e) => warn!(
                    "[DriverPool] Failed to create a new WebDriver session at {}: {e}",
                    endpoint
                ),
            }
        }

        Err(WebDriverError::NotFound(
            "".into(),
            "Failed to create a new WebDriver session at all endpoints.".into(),
        ))
    }

    pub async fn acquire(&self, strategy: AcquireStrategy) -> Result<DriverHandle, PoolError> {
        info!("driver acquire...");
        info!(
            "stats: idle driver: {}, total driver: {}",
            self.stats().await.0,
            self.stats().await.1
        );

        let permit = match strategy {
            AcquireStrategy::FailFast => self
                .semaphore
                .clone()
                .try_acquire_owned()
                .map_err(|_| PoolError::Exhausted)?,
            AcquireStrategy::Wait(Some(timeout)) => {
                tokio::time::timeout(timeout, self.semaphore.clone().acquire_owned())
                    .await
                    .map_err(|_| PoolError::Timeout)?
                    .map_err(|_| PoolError::Exhausted)?
            }
            AcquireStrategy::Wait(None) => self
                .semaphore
                .clone()
                .acquire_owned()
                .await
                .map_err(|_| PoolError::Exhausted)?,
        };

        // permit 수 == 최대 세션 수이므로, permit을 잡았는데 idle이 없으면
        // 죽어서 빠진 세션 자리다. 새로 만들어 채운다.
        let drv = match self.try_take_idle().await {
            Some(drv) => {
                info!("get driver from idle pool.");
                drv
            }
            None => {
                let index = self.stats().await.1;
                let drv = self.spawn_driver(index).await?;
                let mut guard = self.inner.lock().await;
                guard.total += 1;
                info!("[DriverPool] Driver recreated. total={}", guard.total);
                drv
            }
        };

        Ok(DriverHandle {
            driver: Some(drv),
            pool: self.clone(),
            permit: Some(permit),
            broken: false,
        })
    }

    async fn try_take_idle(&self) -> Option<WebDriver> {
        let mut guard = self.inner.lock().await;
        guard.idle.pop_front()
    }

    async fn release(&self, driver: WebDriver, broken: bool) {
        if broken {
            // 세션 종료 후 total 감소 -> 다음 acquire때 새로 생성
            if let Err(e) = driver.quit().await {
                warn!("[DriverPool] Failed to quit broken driver: {e}");
            }
            let mut guard = self.inner.lock().await;
            guard.total -= 1;
            info!(
                "[DriverPool] Driver removed (broken). total={}",
                guard.total
            );
            return;
        }

        let healthy = driver.execute("return 1;", vec![]).await.is_ok();
        if healthy {
            let mut guard = self.inner.lock().await;
            guard.idle.push_back(driver);
        } else {
            let mut guard = self.inner.lock().await;
            guard.total -= 1;
            warn!(
                "[DriverPool] Driver unhealthy on release. Dropped. total={}",
                guard.total
            );
        }
    }

    async fn keepalive_loop(&self, interval: Duration) {
        loop {
            tokio::time::sleep(interval).await;
            {
                let last = self.last_health.read().await;
                if last.elapsed() < interval / 2 {
                    continue;
                }
            }
            {
                let mut last = self.last_health.write().await;
                *last = Instant::now();
            }
            let snapshot = {
                let guard = self.inner.lock().await;
                guard.idle.clone().into_iter().collect::<Vec<_>>()
            };
            for drv in snapshot {
                if drv
                    .execute("return document.hidden;", vec![])
                    .await
                    .is_err()
                {
                    // 깨졌으면 실재 release 시점에 제거되지만 여기서 미리 ping 실패 로그 남김
                    debug!("[DriverPool] Keepalive ping failed for a driver..");
                }
            }
        }
    }

    #[allow(dead_code)]
    pub async fn stats(&self) -> (usize, usize) {
        let guard = self.inner.lock().await;
        (guard.idle.len(), guard.total)
    }
}

impl DriverHandle {
    pub fn driver(&self) -> &WebDriver {
        self.driver.as_ref().unwrap()
    }

    #[allow(dead_code)]
    pub fn mark_broken(mut self) {
        self.broken = true;
    }

    #[allow(dead_code)]
    pub fn take(mut self) -> WebDriver {
        self.driver.take().unwrap()
    }
}

impl Drop for DriverHandle {
    fn drop(&mut self) {
        if let Some(drv) = self.driver.take() {
            let pool = self.pool.clone();
            let broken = self.broken;
            let permit = self.permit.take();
            // 비동기 release
            tokio::spawn(async move {
                pool.release(drv, broken).await;
                drop(permit);
            });
        }
    }
}
