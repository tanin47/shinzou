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

fn gen_string_from_cstring(
    cstring: PointerValue,
    context: &FnContext
) -> Value {
    let strlen = match context.module.get_function("strlen") {
        Some(f) => f,
        None => {
            let fn_type = context.context.i64_type().fn_type(
                &[
                    context.context.i8_type().ptr_type(AddressSpace::Generic).into()
                ],
                false);
            context.module.add_function("strlen", fn_type, Some(Linkage::External))
        },
    };
    let ret_strlen = context.builder.build_call(strlen, &[cstring.into()], "strlen");
    let cstring_size = match ret_strlen.try_as_basic_value().left().unwrap() {
        BasicValueEnum::IntValue(i) => i,
        _ => panic!("unable to get string's length")
    };

    let i8_type = context.context.i8_type();
    let i32_type = context.context.i32_type();

    let string = native::gen_malloc(&context.core.string_struct_type, context);

    let size_with_terminator = cstring_size.const_add(context.context.i32_type().const_int(1, false));
    let array = native::gen_malloc_dynamic_array(&i8_type, size_with_terminator, context);

    let memcpy = match context.module.get_function("llvm.memcpy.p0i8.p0i8.i64") {
        None => {
            context.module.add_function(
                "llvm.memcpy.p0i8.p0i8.i64",
                context.context.i64_type().fn_type(
                    &[
                        i8_type.ptr_type(AddressSpace::Generic).into(),
                        i8_type.ptr_type(AddressSpace::Generic).into(),
                        context.context.i64_type().into(),
                        context.context.i32_type().into(),
                        context.context.bool_type().into()
                    ],
                    false
                ),
                Some(Linkage::External)
            )
        }
        Some(f) => f,
    };

    context.builder.build_call(
        memcpy,
        &[
            array.into(),
            cstring.into(),
            size_with_terminator.into(),
            context.context.i32_type().const_int(4, false).into(),
            context.context.bool_type().const_zero().into()
        ],
        "memcpy"
    );

    let size_pointer = unsafe { context.builder.build_struct_gep(string, 0, "gep") };
    context.builder.build_store(size_pointer, cstring_size);

    let content_pointer = unsafe { context.builder.build_struct_gep(string, 1, "gep") };
    context.builder.build_store(content_pointer, array);

    Value::LlvmString(string)
}


pub fn get_llvm_value(ptr: PointerValue, context: &FnContext) -> BasicValueEnum {
    let first_param_pointer = unsafe { context.builder.build_struct_gep(ptr, 0, "gep_first_param") };
    let first_param = match context.builder.build_load(first_param_pointer, "load_first_param") {
        BasicValueEnum::PointerValue(p) => p,
        x => panic!("Expect BasicValueEnum::PointerValue, found {:?}", x),
    };
    let string_content_pointer = unsafe { context.builder.build_struct_gep(first_param, 1, "gep_content") };
    match context.builder.build_load(string_content_pointer, "load_content") {
        BasicValueEnum::PointerValue(p) => BasicValueEnum::PointerValue(p),
        x => panic!("Expect BasicValueEnum::PointerValue, found {:?}", x),
    }
}

pub fn get_llvm_type(context: &FnContext) -> BasicTypeEnum {
    context.context.i8_type().ptr_type(AddressSpace::Generic).into()
}

pub fn instantiate_from_value(value: BasicValueEnum, class: &tree::Class, context: &FnContext) -> Value {
    let ptr = match value {
        BasicValueEnum::PointerValue(ptr) => ptr,
        x => panic!("Expect BasicValueEnum::PointerValue, found {:?}", x),
    };
    let string_pointer = match gen_string_from_cstring(ptr, context) {
        Value::LlvmString(p) => p,
        x => panic!("Expect Value::String, found {:?}", x),
    };

    let instance_ptr = native::gen_malloc(&class.llvm_struct_type_ref.get().unwrap(), context);
    let first_param_pointer = unsafe { context.builder.build_struct_gep(instance_ptr, 0, "first_param") };
    context.builder.build_store(first_param_pointer, string_pointer);
    Value::Class(instance_ptr, class)
}


pub fn instantiate(instance: &tree::ClassInstance, context: &FnContext) -> Value {
    let value = match gen::gen_expr(&instance.params[0], context) {
        Value::LlvmString(p) => Value::LlvmString(p),
        x => panic!("Expect Value::String, Found {:?}", x),
    };

    let class = match instance.tpe.get().unwrap() {
        tree::ExprType::Class(class_ptr) => {
            let class = unsafe { &*class_ptr };
            if class.name == "@String" {
                class
            } else {
                panic!("Expect @String, found {:?}", class)
            }
        }
        x => panic!("Expect a class, found {:?}", x),
    };

    instantiate_from_value(gen::convert(&value), class, context)
}