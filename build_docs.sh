#!/bin/bash

# Script para build e deploy da documentação multilíngue com Zensical

set -e

echo "🌐 Building B-FAST multilingual documentation..."

# Build versão inglês (principal)
echo "📖 Building English version..."
zensical build -f zensical.toml --clean

# Limpar resíduo em site/pt caso docs/pt tenha sido copiado pelo build raiz
rm -rf site/pt

# Build versão português
echo "📖 Building Portuguese version..."
zensical build -f zensical.pt.toml

echo "✅ Documentation built successfully!"
echo "📁 English: site/"
echo "📁 Português: site/pt/"

# Opcional: servir localmente para teste
if [ "$1" = "--serve" ]; then
    echo "🚀 Serving documentation at http://localhost:8000"
    echo "🌐 English: http://localhost:8000"
    echo "🌐 Português: http://localhost:8000/pt/"
    cd site && python3 -m http.server 8000
fi
