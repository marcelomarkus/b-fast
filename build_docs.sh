#!/bin/bash

# Script para build e deploy da documentação multilíngue

echo "🌐 Building B-FAST multilingual documentation..."

# Build versão inglês (principal)
echo "📖 Building English version..."
mkdocs build --clean

# Build versão português
echo "📖 Building Portuguese version..."
cd docs/pt
mkdocs build --config-file mkdocs.yml --site-dir ../../site/pt
cd ../..

echo "✅ Documentation built successfully!"
echo "📁 English: site/"
echo "📁 Português: site/pt/"

# Opcional: servir localmente para teste
if [ "$1" = "--serve" ]; then
    echo "🚀 Serving documentation at http://localhost:8000"
    echo "🌐 English: http://localhost:8000"
    echo "🌐 Português: http://localhost:8000/pt/"
    cd site && python -m http.server 8000
fi
