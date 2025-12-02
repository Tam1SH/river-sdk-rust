use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

pub type Config = HashMap<String, String>;

pub trait Filter: Send + Sync {
    fn on_request(&mut self) -> Result<(), String> { Ok(()) }
    fn on_response(&mut self) -> Result<(), String> { Ok(()) }
}

pub struct FilterRegistry {
    pub factories: HashMap<String, Box<dyn Fn(Config) -> Box<dyn Filter> + Send + Sync>>,
}

impl FilterRegistry {
    pub fn new() -> Self {
        Self { factories: HashMap::new() }
    }
    
    pub fn add_filter<F, C>(&mut self, name: &str, constructor: C)
    where
        F: Filter + 'static,
        C: Fn(Config) -> F + Send + Sync + 'static,
    {
        self.factories.insert(name.to_string(), Box::new(move |cfg| Box::new(constructor(cfg))));
    }
}


static REGISTRY: OnceLock<Arc<Mutex<FilterRegistry>>> = OnceLock::new();

#[doc(hidden)]
pub fn _init_registry<F>(f: F) -> Arc<Mutex<FilterRegistry>> 
where F: FnOnce(&mut FilterRegistry) {
    REGISTRY.get_or_init(|| {
        let mut r = FilterRegistry::new();
        f(&mut r);
        Arc::new(Mutex::new(r))
    }).clone()
}


#[macro_export]
macro_rules! register_plugin {
    ( $($name:literal => $constructor:expr),+ $(,)? ) => {
        
        fn __river_auto_generated_init(reg: &mut $crate::client::FilterRegistry) {
            $(
                reg.add_filter($name, $constructor);
            )*
        }

        $crate::register_plugin!(__river_auto_generated_init);
    };

    ($init_func:expr) => {
        
        $crate::wit_bindgen::generate!({
        inline: r#"
            package river:client@0.1.0;
            interface filter-factory {
                type config = list<tuple<string, string>>;

                resource filter-instance {
                    on-request: func() -> result<_, string>;
                    on-response: func() -> result<_, string>;
                }

                create: func(name: string, config: config) -> result<filter-instance, string>;
            }

            world plugin {
                export filter-factory;
            }
        "#,
            runtime_path: "::river_sdk::wit_bindgen::rt", 
        });

        struct SdkBridge {
            inner: ::std::cell::RefCell<Box<dyn $crate::client::Filter>>,
        }

        use exports::river::client::filter_factory::{Guest, GuestFilterInstance};

        impl GuestFilterInstance for SdkBridge {
            fn on_request(&self) -> Result<(), String> {
                self.inner.borrow_mut().on_request()
            }
            fn on_response(&self) -> Result<(), String> {
                self.inner.borrow_mut().on_response()
            }
        }

        struct PluginMain;

        impl Guest for PluginMain {
            type FilterInstance = SdkBridge;

            fn create(name: String, config: Vec<(String, String)>) -> Result<crate::exports::river::client::filter_factory::FilterInstance, String> {
                
                let registry = $crate::client::_init_registry($init_func);
                let registry = registry.lock().unwrap();
                
                let cfg_map: ::std::collections::HashMap<String, String> = config.into_iter().collect();

                if let Some(constructor) = registry.factories.get(&name) {
                    Ok(crate::exports::river::client::filter_factory::FilterInstance::new(SdkBridge { inner: ::std::cell::RefCell::new(constructor(cfg_map)) }))
                } else {
                    Err(format!("Filter not found: {}", name))
                }
            }
        }

        export!(PluginMain);
    };
}
