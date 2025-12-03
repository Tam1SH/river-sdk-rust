use river_sdk::{client::{Config, Filter, FilterType}, register_plugin};


struct MyFilter {
    part: String
}

impl MyFilter {
    pub fn new(cfg: Config) -> Self {
        Self {
            part: cfg.get("forbidden").cloned().unwrap_or("".into()),
        }
    }
}

impl Filter for MyFilter {

    fn filter(&mut self) -> Result<bool, String> {

        if context::get_path().contains(&self.part) {
            return Ok(true);
        }
        
        Ok(false)
    }
    
}

struct LogFilter;

impl LogFilter { fn new(_: Config) -> Self { Self } }

impl Filter for LogFilter { 
    fn on_response(&mut self) -> Result<(), String> {
        logger::info(&format!("path: {}", context::get_path()));
        Ok(())
    }
}

register_plugin!(
    "my_filter" => {
        kind: FilterType::Filter,
        factory: MyFilter::new
    },
    "response_logger" => {
        kind: FilterType::Response,
        factory: LogFilter::new
    }
);