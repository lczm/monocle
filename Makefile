MAKEFLAGS += -r
Makefile: ;

# mode goal + file goal, e.g. `make lex examples/foo.c` -> -l examples/foo.c
lex:
	cargo run -- -l $(word 2,$(MAKECMDGOALS))
parse:
	cargo run -- -p $(word 2,$(MAKECMDGOALS))
run:
	cargo run -- -r $(word 2,$(MAKECMDGOALS))

# bare `make <file>` defaults to run; skipped when a mode goal already ran it
%: force
	$(if $(filter lex parse run,$(MAKECMDGOALS)),@:,cargo run -- -r $@)

force: ;