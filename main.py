import numpy as np
from pydantic import BaseModel

import b_fast


class Modelo(BaseModel):
    id: int
    data: str


bf = b_fast.BFast()
m = Modelo(id=7, data="B-FAST ROCKS")
arr = np.array([1.0, 2.0, 3.0])

payload = bf.encode_packed({"modelo": m, "tensor": arr}, compress=True)
print(f"⚡ B-FAST! Payload gerado: {len(payload)} bytes")
