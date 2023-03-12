use std::collections::HashMap;

use inkwell::{
    context::Context,
    targets::{CodeModel, FileType, InitializationConfig, RelocMode, Target, TargetMachine},
    OptimizationLevel, AddressSpace,
};

use crate::parser::AST;

const ARRAY_SIZE: u32 = 1024;

pub fn compile(module_name: &str, funcs: HashMap<Vec<String>, Vec<AST>>, entry: Vec<String>) {
    let context = Context::create();
    let module = context.create_module(module_name);
    let builder = context.create_builder();

    let bool_type = context.bool_type();
    let stack_type = bool_type.array_type(ARRAY_SIZE);

    let stack = module.add_global(stack_type, Some(AddressSpace::default()), "stack");
    stack.set_initializer(&stack_type.const_zero());

    let i64_type = context.i64_type();
    let index = module.add_global(i64_type, Some(AddressSpace::default()), "index");
    index.set_initializer(&i64_type.const_zero());

    let void_type = context.void_type();
    let fn_type = void_type.fn_type(&[], false);

    let mut entry_func = None;

    for (name, asts) in funcs {
        let function = module.add_function(name.join("_").as_str(), fn_type, None);
        let basic_block = context.append_basic_block(function, "entry");
        builder.position_at_end(basic_block);

        if name == entry {
            entry_func = Some(function)
        }

        for ast in asts {
            match ast {
                AST::Left => {
                    let i_p = index.as_pointer_value();
                    let i_v = builder.build_load(i64_type, i_p, "stptrval").into_int_value();
                    let s_p = stack.as_pointer_value();
                    
                    unsafe {
                        let x_p = builder.build_in_bounds_gep(bool_type, s_p, &[i_v], "getst");
                        builder.build_store(x_p, bool_type.const_int(1, false));
                    }

                    // inc i_p
                    builder.build_store(i_p, builder.build_int_add(i_v, i64_type.const_int(1, false), "incrstptr"));
                }
                AST::Right => {
                    let i_p = index.as_pointer_value();
                    let i_v = builder.build_load(i64_type, i_p, "stptrval").into_int_value();
                    let s_p = stack.as_pointer_value();
                    
                    unsafe {
                        let x_p = builder.build_in_bounds_gep(bool_type, s_p, &[i_v], "getst");
                        builder.build_store(x_p, bool_type.const_int(0, false));
                    }

                    // inc i_p
                    builder.build_store(i_p, builder.build_int_add(i_v, i64_type.const_int(1, false), "incrstptr"));
                }
                AST::Print => todo!(),
                AST::Read => todo!(),
                AST::Split(_, _) => todo!(),
                AST::Bracketed(_) => todo!(),
                AST::Id(_) => todo!(),
            }
        }

        builder.build_return(None);
    }

    let function = module.add_function("main", fn_type, None);
    let basic_block = context.append_basic_block(function, "entry");
    builder.position_at_end(basic_block);
    builder.build_call(entry_func.unwrap(), &[], "callentry");
    builder.build_return(None);

    Target::initialize_all(&InitializationConfig::default());
    // use the host machine as the compilation target
    let target_triple = TargetMachine::get_default_triple();
    let cpu = TargetMachine::get_host_cpu_name().to_string();
    let features = TargetMachine::get_host_cpu_features().to_string();

    // make a target from the triple
    let target = Target::from_triple(&target_triple).unwrap();

    let target_machine = target
        .create_target_machine(
            &target_triple,
            &cpu,
            &features,
            OptimizationLevel::Default,
            RelocMode::Default,
            CodeModel::Default,
        )
        .unwrap();

    println!("{}", module.to_string());
    
    let output_filename = String::from(module_name) + ".o";
    target_machine
        .write_to_file(&module, FileType::Object, output_filename.as_ref())
        .map_err(|e| format!("{:?}", e))
        .unwrap();
}
