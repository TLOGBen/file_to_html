
pub mod service {
    pub mod file;
    pub mod html;
    pub mod zip;
    pub mod config_service;
    pub mod traits {
        pub mod i_service;
    }
}

pub mod config {
    pub mod config;
    pub mod ports;
}

pub mod action {
    pub mod cli;
    pub mod interactive;
}

pub mod utils {
    pub mod utils;
}

pub mod facade {
    pub mod conversion_facade;
    pub mod ports {
        pub mod facade_ports;
    }
    pub mod traits {
        pub mod i_conversion;
    }
}

pub mod models {
    pub mod conversion;
    pub mod file;
    pub mod zip;
    pub mod html;
}