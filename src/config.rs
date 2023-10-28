pub struct Config {
    pub is_server: bool,
    pub window: usize,
}

impl Config {
    pub fn parse(args: &[String]) -> Config {
        let mut c = Config {
            is_server: true,
            window: 10,
        };
        for element in args {
            match element.as_str() {
                "--server" => c.is_server = true,
                "--client" => c.is_server = false,
                _ => println!("unrecognized args: {}", element),
            }
        }
        c
    }
}
