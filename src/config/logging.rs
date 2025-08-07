use log::LevelFilter;
use log4rs::{
    append::{
        console::ConsoleAppender,
        rolling_file::{
            policy::compound::{
                roll::fixed_window::FixedWindowRoller, trigger::size::SizeTrigger, CompoundPolicy,
            },
            RollingFileAppender,
        },
    },
    config::{Appender, Logger, Root},
    encode::pattern::PatternEncoder,
    Config,
};
use std::env;

// 스케줄러 이름 상수 정의
const ANNOY_SCHEDULER: &str = "annoy_scheduler";
const NEWS_SCHEDULER: &str = "news_scheduler";
const RSS_INFO_SCHEDULER: &str = "rss_info_update_scheduler";
const RSS_FETCH_AND_NOTIFICATION_SCHEDULER: &str = "rss_fetch_and_notification_scheduler";

pub fn load_logger() {
    // Disable ANSI colors in log4rs output
    env::set_var("RUST_LOG_STYLE", "never");
    // Disable ANSI colors in Rocket CLI output
    env::set_var("ROCKET_CLI_COLORS", "true");
    // Set default log level to "info" if RUST_LOG is not already set
    if env::var("RUST_LOG").is_err() {
        env::set_var("RUST_LOG", "info");
    }

    // Console log
    let stdout = ConsoleAppender::builder()
        .encoder(Box::new(PatternEncoder::new(
            "[{d}] {h({l})} [{T}] - {m} {n}",
        )))
        .build();

    // 스케줄러별 롤링 파일 어펜더 생성 함수
    let create_file_appender = |name: &str| -> RollingFileAppender {
        let log_path = format!("logs/{}.log", name);
        let size_trigger = SizeTrigger::new(10 * 1024 * 1024); // 10MB
        let roller = FixedWindowRoller::builder()
            .build(&format!("logs/{}.log.{{}}", name), 5)
            .unwrap();
        let policy = CompoundPolicy::new(Box::new(size_trigger), Box::new(roller));

        RollingFileAppender::builder()
            .encoder(Box::new(PatternEncoder::new("{d} {h({l})} - {m}{n}")))
            .build(log_path, Box::new(policy))
            .unwrap_or_else(|_| panic!("Failed to create file appender for {}", name))
    };

    // 각 스케줄러별 파일 어펜더
    let annoy_appender = create_file_appender(ANNOY_SCHEDULER);
    let news_appender = create_file_appender(NEWS_SCHEDULER);
    let rss_info_appender = create_file_appender(RSS_INFO_SCHEDULER);
    let rss_fetch_and_notification_scheduler =
        create_file_appender(RSS_FETCH_AND_NOTIFICATION_SCHEDULER);

    // 기본 서버 로그용 파일 어펜더
    let server_appender = create_file_appender("server");

    // Config 생성
    let mut config_builder = Config::builder()
        .appender(Appender::builder().build("stdout", Box::new(stdout)))
        .appender(Appender::builder().build("server_file", Box::new(server_appender)))
        .appender(Appender::builder().build("annoy_file", Box::new(annoy_appender)))
        .appender(Appender::builder().build("news_file", Box::new(news_appender)))
        .appender(Appender::builder().build("rss_info_file", Box::new(rss_info_appender)))
        .appender(Appender::builder().build(
            "rss_fetch_and_notification_file",
            Box::new(rss_fetch_and_notification_scheduler),
        ));

    // 스케줄러별 로거 설정
    config_builder = config_builder
        .logger(
            Logger::builder()
                .appender("annoy_file")
                .appender("stdout")
                .build(ANNOY_SCHEDULER, LevelFilter::Info),
        )
        .logger(
            Logger::builder()
                .appender("news_file")
                .appender("stdout")
                .build(NEWS_SCHEDULER, LevelFilter::Info),
        )
        .logger(
            Logger::builder()
                .appender("rss_info_file")
                .appender("stdout")
                .build(RSS_INFO_SCHEDULER, LevelFilter::Info),
        )
        .logger(
            Logger::builder()
                .appender("rss_fetch_and_notification_file")
                .appender("stdout")
                .build(RSS_FETCH_AND_NOTIFICATION_SCHEDULER, LevelFilter::Info),
        );

    // 루트 로거 설정
    let config = config_builder
        .build(
            Root::builder()
                .appender("stdout")
                .appender("server_file")
                .build(LevelFilter::Info),
        )
        .unwrap();

    log4rs::init_config(config).unwrap();
}

// 스케줄러별 로깅 매크로 정의
#[macro_export]
macro_rules! annoy_info {
    ($($arg:tt)+) => {
        log::info!(target: "annoy_scheduler", $($arg)+)
    };
}

#[macro_export]
macro_rules! annoy_warn {
    ($($arg:tt)+) => {
        log::warn!(target: "annoy_scheduler", $($arg)+)
    };
}

#[macro_export]
macro_rules! annoy_error {
    ($($arg:tt)+) => {
        log::error!(target: "annoy_scheduler", $($arg)+)
    };
}

#[macro_export]
macro_rules! news_info {
    ($($arg:tt)+) => {
        log::info!(target: "news_scheduler", $($arg)+)
    };
}

#[macro_export]
macro_rules! news_warn {
    ($($arg:tt)+) => {
        log::warn!(target: "news_scheduler", $($arg)+)
    };
}

#[macro_export]
macro_rules! news_error {
    ($($arg:tt)+) => {
        log::error!(target: "news_scheduler", $($arg)+)
    };
}

#[macro_export]
macro_rules! rss_info_info {
    ($($arg:tt)+) => {
        log::info!(target: "rss_info_update_scheduler", $($arg)+)
    };
}

#[macro_export]
macro_rules! rss_info_warn {
    ($($arg:tt)+) => {
        log::warn!(target: "rss_info_update_scheduler", $($arg)+)
    };
}

#[macro_export]
macro_rules! rss_info_error {
    ($($arg:tt)+) => {
        log::error!(target: "rss_info_update_scheduler", $($arg)+)
    };
}

#[macro_export]
macro_rules! rss_fetch_and_notification_info {
    ($($arg:tt)+) => {
        log::info!(target: "rss_fetch_and_notification_scheduler", $($arg)+)
    };
}

#[macro_export]
macro_rules! rss_fetch_and_notification_warn {
    ($($arg:tt)+) => {
        log::warn!(target: "rss_fetch_and_notification_scheduler", $($arg)+)
    };
}

#[macro_export]
macro_rules! rss_fetch_and_notification_error {
    ($($arg:tt)+) => {
        log::error!(target: "rss_fetch_and_notification_scheduler", $($arg)+)
    };
}
