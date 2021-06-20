#[derive(Clone, Debug)]
pub struct Config {
    verbose: u8,
    port: u16,
    timeout: u32,
    dbfilename: String,
    logfile: String,
}

impl Config {
    pub fn new() -> Config {
        Config {
            verbose: 0,
            port: 8080,
            timeout: 0,
            dbfilename: "dump.rdb".to_string(),
            logfile: "logger.log".to_string(),
        }
    }

    pub fn get_port(self) -> String {
        return self.port.to_string();
    }
}