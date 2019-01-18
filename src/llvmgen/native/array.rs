use inkwell::AddressSpace;
use inkwell::module::Linkage;
use inkwell::types::BasicTypeEnum;
use inkwell::types::FunctionType;
use inkwell::types::StructType;
use inkwell::values::BasicValueEnum;
use inkwell::values::CallSiteValue;
use inkwell::values::FunctionValue;
use inkwell::values::PointerValue;

use llvmgen::gen;
use llvmgen::gen::FnContext;
use llvmgen::gen::Value;
use semantics::tree;
use llvmgen::native;
use inkwell::types::PointerType;


pub fn get_llvm_value_from_var(var: &tree::Var, context: &FnContext) -> PointerValue {
    let instance_ptr = match context.builder.build_load(var.llvm_ref.get().unwrap(), "load class instance") {
        BasicValueEnum::PointerValue(p) => p,
        _ => panic!()
    };

    get_llvm_value(instance_ptr, context)
}

pub fn get_llvm_value(ptr: PointerValue, context: &FnContext) -> PointerValue {
    let first_param_pointer = unsafe {
        context.builder.build_in_bounds_gep(
            ptr,
            &[context.context.i32_type().const_zero(), context.context.i32_type().const_zero()],
            "gep for the first param of @Array")
    };
    match context.builder.build_load(first_param_pointer, "load the first param of @Array") {
        BasicValueEnum::PointerValue(i) => i,
        x => panic!("Expect BasicValueEnum::PointerValue, found {:?}", x),
    }
}