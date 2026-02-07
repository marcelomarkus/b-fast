# CI/CD Strategy

Estratégia de testes e deploy do B-FAST.

## 🔄 Workflows

### 1. **tests.yml** - Testes Contínuos

Roda em: `push` e `pull_request` em branches `main`, `feat/*`, `fix/*`

#### Jobs Críticos (Bloqueiam Release)

1. **test-python** ✅
   - Testa em Python 3.9, 3.10, 3.11, 3.12
   - Roda pytest em `tests/`
   - Smoke test dos benchmarks
   - **Falha = Bloqueia release**

2. **test-typescript** ✅
   - Build do cliente TypeScript
   - Type checking com `tsc --noEmit`
   - **Falha = Bloqueia release**

3. **code-quality** ✅
   - Black (formatação Python)
   - Ruff (linting Python)
   - Cargo fmt (formatação Rust)
   - Clippy (linting Rust)
   - **Falha = Bloqueia release**

4. **all-tests-passed** ✅ **GATE JOB**
   - Depende de: `test-python`, `test-typescript`, `code-quality`
   - Se qualquer um falhar, este job não roda
   - **Este job é verificado antes do release**

#### Jobs Opcionais (Não Bloqueiam)

5. **test-integration** ⚠️
   - Roda apenas em push para `main`
   - Testa Python → TypeScript (type preservation)
   - Usa `/tmp/` para compartilhar dados
   - **Falha = Não bloqueia release** (pode ser flaky)

### 2. **release.yml** - Deploy Automático

Roda em: `push` de tags `v*` (ex: `v1.1.0`)

#### Fluxo de Deploy

```
Tag v1.1.0 pushed
    ↓
run-tests (chama tests.yml)
    ↓
all-tests-passed ✅
    ↓
┌─────────────────┬──────────────────┐
│                 │                  │
build_wheels      publish_typescript
(PyPI)            (NPM)
```

#### Jobs

1. **run-tests**
   - Chama `tests.yml` completo
   - Se falhar, **para tudo**

2. **build_python_wheels**
   - Depende de: `run-tests`
   - Build para Linux, Windows, macOS
   - Upload de artifacts

3. **publish_python**
   - Depende de: `build_python_wheels`
   - Publica no PyPI

4. **publish_typescript**
   - Depende de: `run-tests`
   - Build e publica no NPM

### 3. **docs.yml** - Documentação

Roda em: `push` para `main`

- Build da documentação multilíngue
- Deploy para GitHub Pages

## 🚦 Proteções

### Branch Protection (main)

- ✅ Require pull request before merging
- ✅ Require approvals (1)
- ✅ Require status checks to pass:
  - `test-python`
  - `test-typescript`
  - `code-quality`
  - `all-tests-passed` ← **GATE**
- ✅ Require branches to be up to date
- ✅ Require conversation resolution

### Rulesets

- ✅ `require_last_push_approval: true`
- ✅ `required_status_checks: ["build"]`
- ⚠️ Bypass list: admins podem mergear

## 📋 Checklist de Release

### 1. Preparação

```bash
# Criar branch de release
git checkout -b release/v1.1.0

# Atualizar versões (ver dev/VERSION_BUMP.md)
# - pyproject.toml
# - python/b_fast/__init__.py
# - client-ts/package.json
# - CHANGELOG.md
# - dev/docs/*.md

# Commit
git add -A
git commit -m "chore: bump version to 1.1.0"
git push origin release/v1.1.0
```

### 2. Pull Request

```bash
# Abrir PR para main
# Aguardar CI passar:
# - ✅ test-python
# - ✅ test-typescript
# - ✅ code-quality
# - ✅ all-tests-passed
```

### 3. Merge e Tag

```bash
# Após aprovação e merge
git checkout main
git pull origin main

# Criar tag
git tag -a v1.1.0 -m "Release v1.1.0"
git push origin v1.1.0
```

### 4. Deploy Automático

O push da tag dispara `release.yml`:

1. ✅ Roda todos os testes novamente
2. ✅ Build wheels Python (Linux, Windows, macOS)
3. ✅ Publica no PyPI
4. ✅ Build e publica no NPM

### 5. Verificação

- [ ] Verificar PyPI: https://pypi.org/project/bfast-py/
- [ ] Verificar NPM: https://www.npmjs.com/package/bfast-client
- [ ] Verificar GitHub Release criado
- [ ] Testar instalação: `pip install bfast-py==1.1.0`
- [ ] Testar instalação: `npm install bfast-client@1.1.0`

## 🔧 Troubleshooting

### Testes falhando no CI mas passando localmente

```bash
# Rodar exatamente como o CI
uv sync --all-extras
uv run maturin develop --release
uv run pytest tests/ -v
```

### Release falhou

1. Verificar logs do workflow
2. Corrigir problema
3. Deletar tag: `git tag -d v1.1.0 && git push origin :refs/tags/v1.1.0`
4. Fazer novo commit de fix
5. Criar nova tag

### Teste de integração falhando

- Não bloqueia release
- Investigar localmente:
  ```bash
  python tests/test_integration_types.py
  cd client-ts && node dist/tests/type-preservation.test.js
  ```

## 📊 Status Badges

Adicionar ao README.md:

```markdown
[![Tests](https://github.com/marcelomarkus/b-fast/actions/workflows/tests.yml/badge.svg)](https://github.com/marcelomarkus/b-fast/actions/workflows/tests.yml)
[![PyPI](https://img.shields.io/pypi/v/bfast-py)](https://pypi.org/project/bfast-py/)
[![NPM](https://img.shields.io/npm/v/bfast-client)](https://www.npmjs.com/package/bfast-client)
```

## 🎯 Filosofia

- **Testes críticos bloqueiam**: Python, TypeScript, qualidade de código
- **Testes flaky não bloqueiam**: Integração Python↔TypeScript
- **Deploy automático**: Tag → Testes → PyPI + NPM
- **Rollback fácil**: Deletar tag e criar nova versão
