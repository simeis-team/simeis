# --- CONFIGURATION DES FLAGS ---
# On applique les flags directement selon l'architecture
export RUSTFLAGS = $(shell if [ `uname -m` = "x86_64" ]; then echo "-C codegen-units=1 -C code-model=kernel"; else echo "-C codegen-units=1"; fi)

# --- CIBLES ---

all: build optimize

# Build standard (Développement)
build:
	cargo build --verbose

# Build de production (utilisé par la CI lors du merge sur main)
release:
	cargo build --release --verbose
	strip target/release/simeis

# Optimisation du binaire de debug
optimize:
	strip target/debug/simeis

# Compilation de la documentation Typst
doc:
	typst compile manuel.typ manuel.pdf

# Vérification statique
check:
	cargo check

# Tests unitaires
test:
	cargo test

# Nettoyage
clean:
	cargo clean
	rm -f manuel.pdf

.PHONY: all build release optimize doc check test clean