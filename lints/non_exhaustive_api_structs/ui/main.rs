#![allow(dead_code)]
#![allow(unused_imports)]

mod api {
    pub struct Unit;

    pub struct Tuple(pub u8);

    pub struct MixedField {
        value: u8,
        pub other: String,
    }

    #[non_exhaustive]
    pub struct Marked {
        pub value: u8,
    }

    #[derive(Default)]
    pub struct AllPublic {
        pub value: u8,
        pub other: String,
    }
}

pub struct OutsideApi {
    pub value: u8,
}

fn main() {}