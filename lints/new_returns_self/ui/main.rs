#![allow(dead_code)]
#![allow(unused_imports)]

use std::fmt::Debug;
use std::sync::Arc;

struct Foo;

impl Foo {
    fn new() -> Self {
        Self
    }
}

struct OptionCtor;

impl OptionCtor {
    fn new() -> Option<Self> {
        None
    }
}

struct ResultCtor;

impl ResultCtor {
    fn new() -> Result<Self, ()> {
        Ok(Self)
    }
}

struct BoxCtor;

impl BoxCtor {
    fn new() -> Box<Self> {
        Box::new(Self)
    }
}

struct ArcCtor;

impl ArcCtor {
    fn new() -> Arc<Self> {
        Arc::new(Self)
    }
}

#[derive(Debug)]
struct ImplCtor;

impl ImplCtor {
    fn new() -> impl Debug {
        Self
    }
}

struct DefaultReturn;

impl DefaultReturn {
    fn new() {}
}

fn main() {}
