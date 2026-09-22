# B-FAST Streamable HTTP Protocol Specification (v1.0)

## 1. Visão Geral
O protocolo de streaming B-FAST (`application/x-bfast-stream`) estende o formato binário B-FAST para comunicação orientada a fluxo contínuo sobre HTTP/1.1 (Chunked Transfer), HTTP/2, HTTP/3 (DATA frames), WebSockets e transporte Streamable HTTP do MCP (Model Context Protocol).

Diferente de protocolos baseados em texto (como SSE e NDJSON) ou protocolos rígidos com compilação de esquemas (como gRPC/Protobuf), o B-FAST Streamable HTTP combina:
- **Framing de Alta Performance:** Delimitação por comprimento Little-Endian sem escaneamento de delimitadores.
- **Zero-Copy Tensor Streaming:** Suporte nativo a arrays numéricos (NumPy / TypedArrays) sem conversão para Base64.
- **Buffer Management Amortizado:** Máquina de estados $O(1)$ para decodificação de fragmentos de rede.
- **Resiliência Industrial:** Trava contra ataques de negação de serviço (`max_frame_size`) e suporte a encerramento gracioso (*End-of-Stream*).

---

## 2. Estrutura do Wire Format

Um stream B-FAST é composto por um **Handshake inicial** (opcional, porém padrão no início do stream HTTP), seguido por uma sequência de **Frames de Dados**:

```
+─────────────────────────────────────────────────────────────+
| Stream Handshake (4 bytes) - 'BS', Version 0x01, Flags 0x00 |
+─────────────────────────────────────────────────────────────+
| Frame 1: [Length: 4B LE][Type: 1B][Flags: 1B][Payload...]  |
+─────────────────────────────────────────────────────────────+
| Frame 2: [Length: 4B LE][Type: 1B][Flags: 1B][Payload...]  |
+─────────────────────────────────────────────────────────────+
| ...                                                         |
+─────────────────────────────────────────────────────────────+
| End-of-Stream Frame (6 bytes): [0x00000000][Type 0x00][0x00]|
+─────────────────────────────────────────────────────────────+
```

### 2.1 Stream Handshake (4 bytes)
Enviado no primeiro chunk da resposta HTTP:
- `Byte 0 - 1`: Magic Number ASCII `'BS'` (`0x42 0x53`) para B-FAST Stream.
- `Byte 2`: Versão do protocolo de streaming (`0x01`).
- `Byte 3`: Flags reservadas (`0x00`).

Se o decodificador encontrar o handshake, valida a compatibilidade de versão. Se ausente, o decodificador faz fallback transparente para processamento direto de frames.

### 2.2 Estrutura do Frame (Length-Prefixed Framing)
Cada frame tem um cabeçalho fixo de 6 bytes:
1. **Payload Length (4 bytes, Uint32 Little-Endian)**: Tamanho exato em bytes do payload do frame (não inclui os 6 bytes do cabeçalho do frame).
2. **Frame Type (1 byte)**:
   - `0x01` (`FRAME_DATA`): Contém um pacote B-FAST válido (comprimido ou não).
   - `0x02` (`FRAME_DICT_DELTA`): Atualização dinâmica do dicionário de strings.
   - `0x00` (`FRAME_EOS`): Sinalização explícita de fim de fluxo (*End-of-Stream*).
3. **Flags (1 byte)**:
   - `Bit 0`: Compressão LZ4 ativada no payload do frame (`0 = desativado, 1 = ativado`).
   - `Bit 1-7`: Reservados.
4. **Payload (N bytes)**: Dados binários do frame correspondentes ao `Payload Length`.

---

## 3. Máquina de Estados do Decodificador

O decodificador (`BFastStreamDecoder`) deve ser resiliente a pacotes TCP fragmentados:
1. **Estado `AWAIT_HANDSHAKE`**: Verifica se os primeiros bytes contêm `'BS'`. Se sim, valida a versão e consome 4 bytes; se não, transiciona diretamente para `AWAIT_FRAME_HEADER`.
2. **Estado `AWAIT_FRAME_HEADER`**: Aguarda pelo menos 6 bytes no buffer. Lê `Payload Length`, `Frame Type` e `Flags`.
   - Se `Payload Length > max_frame_size`, emite erro de segurança e aborta.
   - Se `Frame Type == FRAME_EOS`, finaliza a leitura do stream (sinalização End-of-Stream).
3. **Estado `AWAIT_FRAME_PAYLOAD`**: Aguarda `Payload Length` bytes estarem disponíveis no buffer. Assim que disponíveis:
   - Descompacta o payload (se flag de compressão estiver ativa).
   - Decodifica o pacote B-FAST em objetos nativos (Python / TypeScript).
   - Avança o cursor de leitura e retorna ao estado `AWAIT_FRAME_HEADER`.

---

## 4. Integração HTTP & MCP

- **Media Type Oficial**: `application/x-bfast-stream`
- **FastAPI / Starlette**: Classe `BFastStreamingResponse` que empacota geradores síncronos e assíncronos (`Generator`, `AsyncGenerator`).
- **Model Context Protocol (MCP)**: Content negotiation via header `Accept: application/x-bfast-stream` em conexões Streamable HTTP para envio de grandes massas de dados de ferramentas de IA.
