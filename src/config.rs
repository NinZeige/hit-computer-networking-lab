pub struct Config {
    pub is_server: bool,
    pub window: u8,
    pub seq_siz: u8,
    pub max_timeout: u32,
    pub single_timeout: std::time::Duration,
    pub receive_rate: f64,
    pub send_rate: f64,
}

impl Config {
    pub fn parse(args: &[String]) -> Config {
        let mut c = Self::new();
        for element in args {
            match element.as_str() {
                "--server" => c.is_server = true,
                "--client" => c.is_server = false,
                _ => println!("unrecognized args: {}", element),
            }
        }
        c
    }

    fn new() -> Config {
        Config {
            is_server: true,
            window: 10,
            seq_siz: 100,
            max_timeout: 15,
            single_timeout: std::time::Duration::from_millis(100),
            receive_rate: 0.8,
            send_rate: 1.0,
        }
    }

    pub fn output_filename(&self) -> String {
        if self.is_server {
            "server_output.txt".to_string()
        } else {
            "client_output.txt".to_string()
        }
    }

    pub fn input_filename(&self) -> String {
        if self.is_server {
            "server_input.txt".to_string()
        } else {
            "client_input.txt".to_string()
        }
    }
}
