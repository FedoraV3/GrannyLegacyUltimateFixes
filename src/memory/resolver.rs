use il2cpp_bridge_rs::api;
use il2cpp_bridge_rs::structs::{Class, Method};

pub struct ClassFunction {
    pub address: u64,
    // il2cpp_bridge_rs stuff
    pub func_class: Class,
    pub func_method: Method,
}


pub fn find_function_by_class<S: Into<String>>(
    assembly_name: S,
    class_name: S,
    function_name: S
) -> Option<ClassFunction> {

    let assembly = api::cache::assembly(&*assembly_name.into())?;
    let class = assembly.class(&*class_name.into())?;
    let function = class.method(&*function_name.into())?;

    Some(ClassFunction {
        address: function.address as u64,

        func_class: class,
        func_method: function,
    })
}