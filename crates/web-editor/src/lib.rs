use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    pub fn alert(s: &str);
}

#[wasm_bindgen]
pub fn eval_python(code: &str) {
    use rustpython_vm::Interpreter;
    Interpreter::with_init(Default::default(), |vm| {
        // put this line to add stdlib to the vm
        vm.add_native_modules(rustpython_stdlib::get_module_inits());
    })
    .enter(|vm| {
        dbg!(vm.run_code_string(vm.new_scope_with_builtins(), code, "<...>".to_owned())).ok();
    });
}
