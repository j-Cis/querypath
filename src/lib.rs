// ./src/lib.rs 
// fs-path-query

#[path = "code/gis_cs.rs"]
mod gis_cs;
pub use gis_cs::{SysA, Sys, GetSys};
