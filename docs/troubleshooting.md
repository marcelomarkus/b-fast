# Troubleshooting

## Problemas Comuns

### Erro de Importação
```
ImportError: No module named 'b_fast'
```
**Solução:** Verifique se o B-FAST está instalado:
```bash
pip list | grep b-fast
# Se não aparecer, instale:
pip install b-fast
```

### Erro de Decodificação no Frontend
```
BFastError: Invalid binary format
```
**Causas possíveis:**
1. Servidor não está enviando dados B-FAST
2. Middleware modificando o payload
3. Versões incompatíveis entre backend e frontend

**Solução:**
```typescript
// Verificar content-type
const response = await fetch('/api/data');
console.log(response.headers.get('content-type')); // Deve ser 'application/x-bfast'

// Verificar se é realmente binário
const buffer = await response.arrayBuffer();
console.log(buffer.byteLength); // Deve ter tamanho > 0
```

### Performance Pior que JSON
**Possíveis causas:**
1. Payload muito pequeno (< 512 bytes)
2. Criando novo encoder a cada serialização
3. Convertendo NumPy arrays para listas

**Solução:**
```python
# ✅ Encoder singleton
encoder = b_fast.BFast()

# ✅ Sem compressão para payloads pequenos
if len(data) < 512:
    binary = encoder.encode(data)
else:
    binary = encoder.encode_packed(data, compress=True)
```

### Erro de Compatibilidade Pydantic
```
TypeError: Object of type 'BaseModel' is not serializable
```
**Solução:** Certifique-se de usar Pydantic v2:
```bash
pip install "pydantic>=2.0"
```

## Requisitos do Sistema

### Backend (Python)
- Python 3.8+
- Pydantic 2.0+ (recomendado)
- NumPy 1.20+ (opcional, para arrays)

### Frontend (TypeScript/JavaScript)
- Node.js 14.0+
- Navegadores modernos (ES2018+)

## Logs de Debug

### Habilitando Logs Detalhados
```python
import logging
logging.basicConfig(level=logging.DEBUG)

import b_fast
encoder = b_fast.BFast(debug=True)  # Se disponível
```

### Medindo Performance
```python
import time
import b_fast

encoder = b_fast.BFast()
data = {"test": list(range(1000))}

# Medir serialização
start = time.perf_counter()
binary = encoder.encode_packed(data, compress=True)
end = time.perf_counter()

print(f"Tempo: {(end - start) * 1000:.2f}ms")
print(f"Tamanho: {len(binary)} bytes")
```

## Reportar Problemas

Se encontrar um bug ou problema não listado aqui:

1. Verifique se está usando a versão mais recente
2. Colete informações do sistema:
   ```bash
   python --version
   pip show b-fast
   ```
3. Crie um exemplo mínimo que reproduz o problema
4. Abra uma issue no repositório GitHub
