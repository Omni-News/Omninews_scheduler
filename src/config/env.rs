use dotenv::dotenv;

pub fn load_env() {
    dotenv().ok();
}
