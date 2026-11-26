pub mod fingerprint;
pub mod html;
pub mod table;
pub mod text;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Component {
    pub name: String,
    pub text: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Restore {
    pub components: Vec<Component>,
    pub kept: Vec<(String, String)>,
}
