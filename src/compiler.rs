use std::{collections::HashMap, path::Path, process::Command};

use inkwell::{
    builder::Builder,
    context::Context,
    targets::{CodeModel, FileType, InitializationConfig, RelocMode, Target, TargetMachine},
    types::IntType,
    values::{FunctionValue, GlobalValue},
    AddressSpace, IntPredicate, OptimizationLevel,
};

use crate::parser::AST;

const ARRAY_SIZE: u32 = 1024;

pub fn compile(module_name: &str, funcs: HashMap<Vec<String>, Vec<AST>>, entry: Vec<String>) {
    let context = Context::create();
    let module = context.create_module(module_name);
    let builder = context.create_builder();

    // initialise types and globals

    let bool_type = context.bool_type();
    let stack_type = bool_type.array_type(ARRAY_SIZE);

    let stack = module.add_global(stack_type, Some(AddressSpace::default()), "stack");
    stack.set_initializer(&stack_type.const_zero());

    let i64_type = context.i64_type();
    let i32_type = context.i32_type();
    let index = module.add_global(i64_type, Some(AddressSpace::default()), "index");
    index.set_initializer(&i64_type.const_zero());

    let chr_type = context.i32_type();

    let void_type = context.void_type();
    let fn_type = void_type.fn_type(&[], false);

    // external functions

    let pc_fn_type = void_type.fn_type(&[i32_type.into()], false);
    let pc_fn_val = module
        .get_function("putchar")
        .unwrap_or(module.add_function("putchar", pc_fn_type, None));

    let gc_fn_type = i32_type.fn_type(&[], false);
    let gc_fn_val = module
        .get_function("getchar")
        .unwrap_or(module.add_function("getchar", gc_fn_type, None));

    // internal functions

    let dec_func = module.add_function("decri", fn_type, None);
    {
        let basic_block = context.append_basic_block(dec_func, "entry");
        builder.position_at_end(basic_block);

        let i_p = index.as_pointer_value();

        // dec i_p
        let i_ov = builder.build_load(i64_type, i_p, "").into_int_value();

        let if_z = context.append_basic_block(dec_func, "");
        let el = context.append_basic_block(dec_func, "");

        builder.build_conditional_branch(
            builder.build_int_compare(IntPredicate::EQ, i_ov, i64_type.const_zero(), ""),
            if_z,
            el,
        );

        builder.position_at_end(if_z);

        builder.build_return(None);

        builder.position_at_end(el);

        let i_v = builder.build_int_sub(i_ov, i64_type.const_int(1, false), "");
        builder.build_store(i_p, i_v);
        builder.build_return(None);
    }

    let inc_func = module.add_function("incri", fn_type, None);
    {
        let basic_block = context.append_basic_block(inc_func, "entry");
        builder.position_at_end(basic_block);

        let i_p = index.as_pointer_value();

        let i_v = builder.build_load(i64_type, i_p, "").into_int_value();

        let i_nv = builder.build_int_add(i_v, i64_type.const_int(1, false), "");

        let if_l = context.append_basic_block(inc_func, "");
        let el = context.append_basic_block(inc_func, "");

        builder.build_conditional_branch(
            builder.build_int_compare(
                IntPredicate::UGE,
                i_nv,
                i64_type.const_int(ARRAY_SIZE.into(), false),
                "",
            ),
            if_l,
            el,
        );

        builder.position_at_end(if_l);
        // PANIC!!!!!!!
        builder.build_return(None);

        builder.position_at_end(el);
        builder.build_store(i_p, i_nv);
        builder.build_return(None);
    }

    let print_func = module.add_function("print", fn_type, None);
    {
        let basic_block = context.append_basic_block(print_func, "entry");
        builder.position_at_end(basic_block);

        let s_p = stack.as_pointer_value();
        let mut acc = chr_type.const_int(0, false);

        for _ in 0..8 {
            builder.build_call(dec_func, &[], "");
            let i_p = index.as_pointer_value();
            let i_v = builder.build_load(i64_type, i_p, "").into_int_value();

            unsafe {
                let x_p = builder.build_in_bounds_gep(bool_type, s_p, &[i_v], "");
                let this_bit = builder
                    .build_load(bool_type, x_p, "")
                    .into_int_value()
                    .const_cast(chr_type, false);

                acc = builder.build_int_mul(acc, chr_type.const_int(2, false), "");
                acc = builder.build_int_add(acc, this_bit, "");
            }
        }

        builder.build_call(pc_fn_val, &[acc.into()], "");
        builder.build_return(None);
    }

    let read_func = module.add_function("read", fn_type, None);
    {
        let basic_block = context.append_basic_block(read_func, "entry");
        builder.position_at_end(basic_block);

        let s_p = stack.as_pointer_value();
        let mut acc = builder.build_call(gc_fn_val, &[], "").try_as_basic_value().unwrap_left().into_int_value();

        for _ in 0..8 {
            let i_p = index.as_pointer_value();
            let i_v = builder.build_load(i64_type, i_p, "").into_int_value();

            unsafe {
                let x_p = builder.build_in_bounds_gep(bool_type, s_p, &[i_v], "");

                builder.build_store(x_p, builder.build_int_truncate(acc, bool_type, ""));
            }

            acc = builder.build_right_shift(acc, i32_type.const_int(1, false), false, "");
            builder.build_call(inc_func, &[], "");
        }

        builder.build_return(None);
    }

    let mut entry_func = None;
    let mut func_defs = HashMap::new();

    for (name, _) in &funcs {
        let function = module.add_function(name.join("_").as_str(), fn_type, None);
        func_defs.insert(name.clone(), function);
    }

    for (name, asts) in funcs {
        let function = func_defs[&name];
        let basic_block = context.append_basic_block(function, "entry");
        builder.position_at_end(basic_block);

        if name == entry {
            entry_func = Some(function)
        }

        build_ast(
            asts,
            &Env {
                builder: &builder,
                index: &index,
                stack: &stack,
                bool_type: bool_type,
                i64_type: i64_type,
                print_func: print_func,
                function: function,
                context: &context,
                dec_func: dec_func,
                inc_func: inc_func,
                func_defs: &func_defs,
                read_func: read_func,
            },
        );

        builder.build_return(None);
    }

    let function = module.add_function("main", fn_type, None);
    let basic_block = context.append_basic_block(function, "entry");
    builder.position_at_end(basic_block);
    builder.build_call(entry_func.unwrap(), &[], "");
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
            OptimizationLevel::Aggressive,
            RelocMode::Default,
            CodeModel::Default,
        )
        .unwrap();

    println!("{}", module.to_string());

    let s = module_name.to_string() + ".o";
    let output_filename = Path::new(&s);
    target_machine
        .write_to_file(&module, FileType::Object, output_filename)
        .map_err(|e| format!("{:?}", e))
        .unwrap();

    let mut cmd = Command::new("clang");
    cmd.arg(output_filename)
        .arg("-o")
        .arg(Path::new(module_name));
}

struct Env<'a> {
    builder: &'a Builder<'a>,
    index: &'a GlobalValue<'a>,
    stack: &'a GlobalValue<'a>,
    bool_type: IntType<'a>,
    i64_type: IntType<'a>,
    print_func: FunctionValue<'a>,
    read_func: FunctionValue<'a>,
    inc_func: FunctionValue<'a>,
    dec_func: FunctionValue<'a>,
    function: FunctionValue<'a>,
    context: &'a Context,
    func_defs: &'a HashMap<Vec<String>, FunctionValue<'a>>,
}

fn build_ast(asts: Vec<AST>, env: &Env) {
    for ast in asts {
        match ast {
            AST::Left => {
                let i_p = env.index.as_pointer_value();
                let i_v = env
                    .builder
                    .build_load(env.i64_type, i_p, "")
                    .into_int_value();
                let s_p = env.stack.as_pointer_value();

                unsafe {
                    let x_p = env
                        .builder
                        .build_in_bounds_gep(env.bool_type, s_p, &[i_v], "");
                    env.builder
                        .build_store(x_p, env.bool_type.const_int(1, false));
                }

                // inc i_p
                env.builder.build_call(env.inc_func, &[], "");
            }
            AST::Right => {
                let i_p = env.index.as_pointer_value();
                let i_v = env
                    .builder
                    .build_load(env.i64_type, i_p, "")
                    .into_int_value();
                let s_p = env.stack.as_pointer_value();

                unsafe {
                    let x_p = env
                        .builder
                        .build_in_bounds_gep(env.bool_type, s_p, &[i_v], "");
                    env.builder
                        .build_store(x_p, env.bool_type.const_int(0, false));
                }

                // inc i_p
                env.builder.build_call(env.inc_func, &[], "");
            }
            AST::Print => {
                env.builder.build_call(env.print_func, &[], "");
            }
            AST::Read => {
                env.builder.build_call(env.read_func, &[], "");
            }
            AST::Split(l, r) => {
                let s_p = env.stack.as_pointer_value();

                env.builder.build_call(env.dec_func, &[], "");
                let i_p = env.index.as_pointer_value();

                // dec i_p
                let i_v = env
                    .builder
                    .build_load(env.i64_type, i_p, "")
                    .into_int_value();

                let left_block = env.context.append_basic_block(env.function, "");
                let right_block = env.context.append_basic_block(env.function, "");
                let end_block = env.context.append_basic_block(env.function, "");

                unsafe {
                    let x_p = env
                        .builder
                        .build_in_bounds_gep(env.bool_type, s_p, &[i_v], "");
                    let x_v = env
                        .builder
                        .build_load(env.bool_type, x_p, "")
                        .into_int_value();
                    env.builder.build_conditional_branch(
                        env.builder.build_int_compare(
                            inkwell::IntPredicate::EQ,
                            x_v,
                            env.bool_type.const_zero(),
                            "",
                        ),
                        right_block,
                        left_block,
                    );

                    // if left

                    env.builder.position_at_end(left_block);
                    build_ast(l, env);
                    env.builder.build_unconditional_branch(end_block);

                    // if right

                    env.builder.position_at_end(right_block);
                    build_ast(r, env);
                    env.builder.build_unconditional_branch(end_block);

                    env.builder.position_at_end(end_block);
                }
            }
            AST::Bracketed(c) => build_ast(c, env),
            AST::Id(id) => {
                env.builder.build_call(env.func_defs[&id], &[], "");
            }
        }
    }
}
