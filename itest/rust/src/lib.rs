use gdext_builtin::gdext_init;

gdext_init!(itest_init, |init: &mut gdext_builtin::InitOptions| {
    init.register_init_function(gdext_builtin::InitLevel::Scene, || {
        //register_class::<IntegrationTests>();
        println!("Run Godot integration tests...");


        println!("Tests finished");
    });
});
