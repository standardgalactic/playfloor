use std::io::{self, Write};
use spherepop::{
    compile, eval_closed, eval_with_options,
    infer_open, infer_closed, parse,
    CollapseRule, Context, Machine, RefusalReason, Symbol, TypeMode,
};

fn main() {
    println!("Spherepop v0.2 — history-first process calculus");
    println!();
    println!("Primitives: pop <x>  refuse <x> [reason]  collapse <x> [rule]  bind <a> <b>");
    println!("Collapse rules: id | last_write | accumulate | proj(label) | <name>");
    println!("Refusal reasons: violation(<c>) | explicit(<msg>)");
    println!("Lambda: \\x : T -> body   let x = val in body   seq[a; b; c]");
    println!();
    println!("Commands:");
    println!("  :type <term>         infer type (open-world)");
    println!("  :type! <term>        infer type (closed-world — rejects free vars)");
    println!("  :compile <term>      show event IR");
    println!("  :observe <rule>      show observable state under rule");
    println!("  :options [a b c]     set/show option space");
    println!("  :mode [open|closed]  get/set type checking mode");
    println!("  :quit");
    println!();

    let stdin = io::stdin();
    let mut options: Vec<String> = Vec::new();
    let mut mode = TypeMode::OpenWorld;
    let mut last_world: Option<spherepop::World> = None;

    loop {
        print!("sp{}> ", if mode == TypeMode::ClosedWorld { "!" } else { "" });
        io::stdout().flush().unwrap();

        let mut line = String::new();
        if stdin.read_line(&mut line).unwrap() == 0 { break; }
        let line = line.trim();
        if line.is_empty() { continue; }

        if line == ":quit" || line == ":q" { break; }

        if let Some(rest) = line.strip_prefix(":mode") {
            let rest = rest.trim();
            if rest.is_empty() {
                println!("current mode: {}", mode);
            } else if rest == "open" {
                mode = TypeMode::OpenWorld;
                println!("mode: open-world (free variables treated as Unit)");
            } else if rest == "closed" {
                mode = TypeMode::ClosedWorld;
                println!("mode: closed-world (free variables are type errors)");
            }
            continue;
        }

        if let Some(rest) = line.strip_prefix(":options") {
            let rest = rest.trim();
            if rest.is_empty() {
                println!("options: {:?}", options);
            } else {
                options = rest.split_whitespace().map(|s| s.to_string()).collect();
                println!("option space: {:?}", options);
            }
            continue;
        }

        if let Some(rest) = line.strip_prefix(":type! ") {
            match parse(rest) {
                Err(e) => println!("parse error: {}", e),
                Ok(term) => match infer_closed(&term) {
                    Ok(ty) => println!("  : {} [closed-world]", ty),
                    Err(e) => println!("type error: {}", e),
                },
            }
            continue;
        }

        if let Some(rest) = line.strip_prefix(":type ") {
            match parse(rest) {
                Err(e) => println!("parse error: {}", e),
                Ok(term) => match infer_open(&term) {
                    Ok(ty) => println!("  : {}", ty),
                    Err(e) => println!("type error: {}", e),
                },
            }
            continue;
        }

        if let Some(rest) = line.strip_prefix(":compile ") {
            match parse(rest) {
                Err(e) => println!("parse error: {}", e),
                Ok(term) => println!("{}", compile(&term)),
            }
            continue;
        }

        if let Some(rest) = line.strip_prefix(":observe") {
            if let Some(w) = &last_world {
                let rest = rest.trim();
                let rule = match rest {
                    "last_write" => CollapseRule::LastWrite,
                    "accumulate" => CollapseRule::Accumulate,
                    r if r.starts_with("proj(") => {
                        let label = r.trim_start_matches("proj(").trim_end_matches(')');
                        CollapseRule::Projection(Symbol::new(label))
                    }
                    _ => CollapseRule::Identity,
                };
                println!("  {}", w.observe(&rule));
            } else {
                println!("  (no world yet — evaluate a term first)");
            }
            continue;
        }

        // Normal evaluation
        match parse(line) {
            Err(e) => println!("parse error: {}", e),
            Ok(term) => {
                let ctx = Context::new().with_mode(mode);
                match spherepop::infer(&ctx, &term) {
                    Err(e) => { println!("type error: {}", e); println!(); continue; }
                    Ok(ty) => {
                        let syms: Vec<Symbol> = options.iter().map(|s| Symbol::new(s.as_str())).collect();
                        let eval_res = if syms.is_empty() {
                            eval_closed(&term)
                        } else {
                            eval_with_options(&term, syms.clone())
                        };
                        match eval_res {
                            Err(e) => println!("eval error: {}", e),
                            Ok((res, world)) => {
                                println!("  value    : {}", res.value);
                                println!("  type     : {}", ty);
                                println!("  history  : {}", world.history);
                                if !world.options.is_empty() {
                                    println!("  options  : {}", world.options);
                                }
                                if world.history.refuse_count() > 0 {
                                    for ev in world.history.events() {
                                        if let spherepop::core::event::Event::Refuse { target, reason } = ev {
                                            println!("  refused  : {} ∵ {}", target, reason);
                                        }
                                    }
                                }
                                if !res.admissible {
                                    println!("  ⚠ inadmissible branch");
                                }
                                // Compile and verify history equivalence
                                let block = compile(&term);
                                let mut machine = Machine::new(syms);
                                match machine.run(&block) {
                                    Err(e) => println!("  vm error : {}", e),
                                    Ok(_) => {
                                        if world.history == machine.world.history {
                                            println!("  ✓ interp ≡ compiler (same history)");
                                        } else {
                                            println!("  ✗ history mismatch!");
                                        }
                                    }
                                }
                                // Show identity observable state
                                let obs = world.observe_identity();
                                if !obs.entries.is_empty() {
                                    println!("  observe  : {}", obs);
                                }
                                last_world = Some(world);
                            }
                        }
                    }
                }
                println!();
            }
        }
    }
    println!("Goodbye.");
}
