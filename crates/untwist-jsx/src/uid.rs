use untwist_core::pass::Cx;

pub const COMPONENT: &str = "Component";
const BABEL_PREFIX: &str = "_";

pub fn component_name(conservative: bool, cx: &mut Cx<'_, '_>) -> Option<String> {
    let base = match conservative {
        true => format!("{BABEL_PREFIX}{COMPONENT}"),
        false => COMPONENT.to_string(),
    };
    cx.invent_name(&base)
}
