B-FAST Specification (v1.0)
Binary Fast Adaptive Serialization Transfer é um formato de serialização binária focado em performance extrema, redução de redundância e suporte nativo a tipos complexos.

1. Estrutura do Stream
Um pacote B-FAST consiste em três partes principais:
Seção	Comprimento	Descrição
Header	6 bytes + var	Metadados do arquivo e Tabela de Strings.
Payload	Variável	Os dados serializados em formato de tags.
Checksum	4 bytes (opcional)	CRC32 para integridade de dados.

1.1 Header Detalhado
    1.Magic Number (2 bytes): 0x42 0x46 (ASCII para 'BF').
    2.Flags (1 byte): - Bit 0: Compressão LZ4 (0=Off, 1=On).
        Bit 1: Endianness (0=Little, 1=Big).
    3.Version (1 byte): Atualmente 0x01.
    4.String Table Count (2 bytes): Quantidade de strings únicas no dicionário (Uint16 LE).
    5.String Table Data: Sequência de [Length (1 byte)][UTF-8 Data].


2. Tipos de Dados (Tags)
O B-FAST usa um sistema de tags de 1 byte. O primeiro nibble (4 bits) geralmente define a categoria do tipo.

2.1 Primitivos e Bit-Packing
Tag	Tipo	Descrição
0x10	Null	Representa None ou null.
0x20	Bool False	Booleano falso.
0x21	Bool True	Booleano verdadeiro.
0x3x	Small Int	Onde x (0-F) é o valor do inteiro (0 a 15).
0x38	Int64	Seguido por 8 bytes (Little Endian).

2.2 Coleções
    - 0x70 (Object Start): Inicia um mapa. Seguido por pares de [StringID (u32)][Value].
    - 0x7F (Object End): Finaliza o mapa.
    - 0x60 (List Start): Seguido por [Length (u32)] e então N valores.

2.3 Tipos Complexos (High Performance)
    - 0x80 (UUID): Seguido por 16 bytes brutos (sem hifens).
    - 0x81 (DateTime): Seguido por 8 bytes (Int64 Unix Timestamp).
    - 0x50 (Raw String): Para strings que não estão na tabela. Seguido por [Length (u32)][UTF-8 Data].


3. Lógica de String Interning
Para reduzir o tamanho do payload, chaves de objetos repetitivas devem ser enviadas na String Table do Header.
    - No corpo do Payload, ao encontrar a tag de objeto (0x70), o Decoder deve ler os próximos 4 bytes como um índice (u32) que aponta para a posição da string na tabela global.

4. Compressão (Opcional)
Se a Flag de Compressão (Header[2] & 0x01) estiver ativa:
    1. O Header é lido normalmente.
    2. O restante do stream (Payload) deve ser passado pelo descompressor LZ4 antes do parsing das tags.

