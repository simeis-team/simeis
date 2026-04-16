# --- CONFIGURATION DES FLAGS --- [cite: 15, 16]
# On détecte l'architecture pour permettre le build local sur Mac (AArch64)
# tout en respectant les consignes du TP pour l'environnement final.
ARCH := $(shell uname -m)

# Flags de base imposés par le sujet [cite: 17, 18]
BASE_RUSTFLAGS = -C codegen-units=1

# Ajout du code-model=kernel seulement si on n'est pas sur ARM (Mac M1/M2) [cite: 17]
ifeq ($(ARCH), x86_64)
    export RUSTFLAGS = $(BASE_RUSTFLAGS) -C code-model=kernel
else
    # Sur Mac, on garde BASE_RUSTFLAGS pour que ça compile, 
    # mais la CI utilisera le modèle kernel sur les serveurs Linux.
    export RUSTFLAGS = $(BASE_RUSTFLAGS)
endif

# --- VARIABLES ---
BINARY_NAME = simeis
BINARY_PATH = target/debug/$(BINARY_NAME)
RELEASE_PATH = target/release/$(BINARY_NAME)
MANUAL_SRC = manuel.typ
MANUAL_OUT = manuel.pdf

# --- CIBLES PRINCIPALES ---

# Cible par défaut : construit et optimise [cite: 13, 20]
all: build optimize

# 1. Utiliser Make pour construire avec cargo en mode verbeux [cite: 13, 19]
build:
	cargo build --verbose

# 2. Compilation mode release (utilisée par la CI) 
release:
	cargo build --release --verbose
	strip $(RELEASE_PATH)

# 3. Optimisation de la taille via strip 
optimize:
	@if [ -f $(BINARY_PATH) ]; then \
		strip $(BINARY_PATH); \
		echo "Optimisation terminée pour $(BINARY_PATH)"; \
	else \
		echo "Erreur : Binaire non trouvé. Lancez 'make build' d'abord."; \
	fi

# --- AUTRES CIBLES [cite: 21] ---

# Construire le manuel avec typst 
doc:
	typst compile $(MANUAL_SRC) $(MANUAL_OUT)

# Check du code (statique) [cite: 23]
check:
	cargo check

# Lancer les tests unitaires [cite: 24]
test:
	cargo test

# Nettoyer l'environnement [cite: 25]
clean:
	cargo clean
	rm -f $(MANUAL_OUT)

# Évite les conflits avec des fichiers de même nom
.PHONY: all build release optimize doc check test clean