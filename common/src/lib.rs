#![no_std]

#[macro_use]
extern crate serde_derive;

#[derive(Serialize, Deserialize, Debug)]
pub struct Request {
    pub val: u16,
}
