use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

#[derive(Debug, Clone, Copy)]
pub enum FilterType {
    Request,
    Response,
    Filter,
}

pub type Config = HashMap<String, String>;

pub trait Filter: Send + Sync {
    fn on_request(&mut self) -> Result<(), String> { Ok(()) }
    fn on_response(&mut self) -> Result<(), String> { Ok(()) }
    fn filter(&mut self) -> Result<bool, String> { Ok(false) }
}


pub struct FilterDescriptor {
    pub kind: FilterType,
    pub factory: Box<dyn Fn(Config) -> Box<dyn Filter> + Send + Sync>,
}

#[derive(Default)]
pub struct FilterRegistry {
    pub factories: HashMap<String, FilterDescriptor>,
}

impl FilterRegistry {
    
    pub fn add_filter<F, C>(&mut self, name: &str, kind: FilterType, constructor: C)
    where
        F: Filter + 'static,
        C: Fn(Config) -> F + Send + Sync + 'static,
    {
        self.factories.insert(
            name.to_string(),
            FilterDescriptor {
                kind,
                factory: Box::new(move |cfg| Box::new(constructor(cfg))),
            },
        );
    }
}

static REGISTRY: OnceLock<Arc<Mutex<FilterRegistry>>> = OnceLock::new();

#[doc(hidden)]
pub fn _init_registry<F>(f: F) -> Arc<Mutex<FilterRegistry>> 
where F: FnOnce(&mut FilterRegistry) {
    REGISTRY.get_or_init(|| {
        let mut r = FilterRegistry::default();
        f(&mut r);
        Arc::new(Mutex::new(r))
    }).clone()
}

#[macro_export]
macro_rules! register_plugin {
    
    ( 
        $(
            $name:literal => {
                kind: $kind:expr,
                factory: $constructor:expr $(,)?
            }
        ),+ $(,)? 
    ) => {
        
        fn __river_auto_generated_init(reg: &mut $crate::client::FilterRegistry) {
            $(
                reg.add_filter($name, $kind, $constructor);
            )*
        }

        $crate::register_plugin!(__river_auto_generated_init);
    };

    ($init_func:expr) => {
        
        $crate::wit_bindgen::generate!({
            inline: r#"
            package river:client@0.1.0;

            interface logger {
              info: func(msg: string);
              warn: func(msg: string);
              error: func(msg: string);
              debug: func(msg: string);
            }

            interface context {
              get-path: func() -> string;
            }

            interface filter-factory {
              type config = list<tuple<string, string>>;

              enum filter-type {
                request,
                response,
                filter
              }

              resource filter-instance {
                on-request: func() -> result<_, string>;
                on-response: func() -> result<_, string>;
              }

              create: func(name: string, config: config) -> result<option<tuple<filter-instance, filter-type>>, string>;
            }

            world client {
                import context;
                import logger;
                export filter-factory;
            }
            "#,
            
            runtime_path: "::river_sdk::wit_bindgen::rt", 
        });

        struct SdkBridge {
            inner: ::std::cell::RefCell<Box<dyn $crate::client::Filter>>,
        }

        pub mod context {
            pub fn get_path() -> String { crate::river::client::context::get_path() }
        }

        pub mod logger {
            pub fn info(msg: &str) { crate::river::client::logger::info(msg) }
            pub fn error(msg: &str) { crate::river::client::logger::error(msg) }
            pub fn warn(msg: &str) { crate::river::client::logger::warn(msg) }
            pub fn debug(msg: &str) { crate::river::client::logger::debug(msg) }
        }

        use exports::river::client::filter_factory::{Guest, GuestFilterInstance};
        
        use exports::river::client::filter_factory::FilterType as WitFilterType;

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
            
            fn create(name: String, config: Vec<(String, String)>) -> Result<Option<(crate::exports::river::client::filter_factory::FilterInstance, WitFilterType)>, String> {
                
                let registry = $crate::client::_init_registry($init_func);
                let registry = registry.lock().unwrap();
                
                let cfg_map: ::std::collections::HashMap<String, String> = config.into_iter().collect();

                if let Some($crate::client::FilterDescriptor { kind, factory }) = registry.factories.get(&name) {
                    
                    let wit_type = match kind {
                        $crate::client::FilterType::Request => WitFilterType::Request,
                        $crate::client::FilterType::Response => WitFilterType::Response,
                        $crate::client::FilterType::Filter => WitFilterType::Filter,
                    };

                    let instance = crate::exports::river::client::filter_factory::FilterInstance::new(
                        SdkBridge { 
                            inner: ::std::cell::RefCell::new(factory(cfg_map)) 
                        }
                    );

                    Ok(Some((instance, wit_type)))
                } else {
                    Err(format!("Filter not found: {}", name))
                }
            }
        }

        export!(PluginMain);
    };
}