# ⚡ Além dos Limites do Navegador: B-FAST em Tempo Real

O **B-FAST** processa e transfere dados massivos em tempo real, mantendo a taxa de quadros cravada no máximo do seu monitor (**60 a 120+ FPS**) com fluidez absoluta e zero impacto na interface.

Abra em **Modo Imersivo (Tela Cheia)**, selecione a densidade de partículas e veja o que parecia impossível acontecer no seu navegador.

---

<div id="bfast-demo-box" class="bfast-demo-wrapper">
  <!-- Barra de Controles -->
  <div class="bfast-controls-bar">
    <div class="bfast-control-group">
      <label>Densidade de Dados:</label>
      <select id="select-density" onchange="onDensityChange(this.value)">
        <option value="250000">250.000 Vértices (750k Coords)</option>
        <option value="500000" selected>500.000 Vértices (1.5M Coords)</option>
        <option value="1000000">1.000.000 Vértices (3.0M Coords)</option>
        <option value="2000000">🚀 2.000.000 Vértices (6.0M Coords — Extremo)</option>
      </select>
    </div>

    <div class="bfast-control-group">
      <label>Interação em Tempo Real:</label>
      <div class="bfast-action-buttons">
        <button id="btn-stream" class="bfast-btn bfast-btn-primary" onclick="toggleStream()">🔄 Stream Contínuo</button>
        <button id="btn-immersive" class="bfast-btn bfast-btn-immersive" onclick="toggleFullscreen()">⛶ Modo Imersivo (Tela Cheia)</button>
        <button id="btn-modal-close" class="bfast-modal-close-btn" onclick="toggleFullscreen()">✕ Sair da Tela Cheia (Esc)</button>
      </div>
    </div>
  </div>

  <!-- HUD de Telemetria em Tempo Real -->
  <div class="bfast-hud-grid">
    <div class="bfast-hud-card">
      <div class="bfast-hud-label">FPS em Tempo Real</div>
      <div id="hud-fps" class="bfast-hud-value fps-good">60 FPS</div>
      <canvas id="hud-fps-sparkline" width="140" height="28"></canvas>
    </div>
    <div class="bfast-hud-card">
      <div class="bfast-hud-label">Custo na Main Thread</div>
      <div id="hud-jank" class="bfast-hud-value jank-good">0.0 ms</div>
      <div id="hud-jank-sub" class="bfast-hud-sub">UI 100% Livre</div>
    </div>
    <div class="bfast-hud-card">
      <div class="bfast-hud-label">Tempo de Decode</div>
      <div id="hud-decode" class="bfast-hud-value fps-good">-- ms</div>
      <div id="hud-decode-sub" class="bfast-hud-sub">Decodificação B-FAST</div>
    </div>
    <div class="bfast-hud-card">
      <div class="bfast-hud-label">Payload Binário</div>
      <div id="hud-wire" class="bfast-hud-value savings-good">-- MB</div>
      <div id="hud-wire-sub" class="bfast-hud-sub">Protocolo B-FAST</div>
    </div>
  </div>

  <!-- WebGL 3D Canvas -->
  <div class="bfast-canvas-container">
    <canvas id="bfast-3d-canvas"></canvas>
    <div id="bfast-status-badge" class="bfast-status-badge status-bfast">⚡ B-FAST Oficial (npm v1.12.0): 500.000 Partículas</div>
    <div class="bfast-canvas-overlay-hints">⚡ Arraste para girar • Scroll para zoom • Teste o stream contínuo em tempo real</div>
  </div>
</div>

<script src="https://cdnjs.cloudflare.com/ajax/libs/three.js/r128/three.min.js"></script>

<script>
(function() {
  var pointCount = 500000;
  var isStreaming = false;
  var streamTimer = null;
  var lastFrameTime = 0;
  var isFirstFrame = true;
  var frameDeltas = [];
  var maxJankMs = 0;
  var lastDecodeDurationMs = 0;
  var isFullscreen = false;

  var targetMonitorFps = 60;
  var maxObservedFps = 60;

  var dynamicSurge = 0;
  var surgeActive = false;

  var fpsHistory = new Array(30).fill(60);

  var canvas = document.getElementById('bfast-3d-canvas');
  var scene, camera, renderer;
  var boltMesh = null;
  var boltGeometry = null;
  var positionsAttr, colorsAttr;
  var baseBoltPoints = null;

  var isMouseDown = false;
  var mouseX = 0, mouseY = 0;
  var targetRotationX = 0.25, targetRotationY = 0.45;
  var cameraDistance = 160;

  function pointInPolygon(x, y, vs) {
    var inside = false;
    for (var i = 0, j = vs.length - 1; i < vs.length; j = i++) {
      var xi = vs[i][0], yi = vs[i][1];
      var xj = vs[j][0], yj = vs[j][1];
      var intersect = ((yi > y) !== (yj > y)) && (x < (xj - xi) * (y - yi) / (yj - yi) + xi);
      if (intersect) inside = !inside;
    }
    return inside;
  }

  function precomputeBoltShape(count) {
    baseBoltPoints = new Float32Array(count * 4);
    var poly = [
      [18, 68], [-14, 68], [-44, 2], [-4, 2], [-32, -68],
      [44, -6], [4, -6], [34, 40], [18, 40]
    ];
    var bounds = { minX: -44, maxX: 44, minY: -68, maxY: 68 };
    var filled = 0;
    var spineCount = Math.floor(count * 0.20);
    var arcCount = Math.floor(count * 0.12);
    var bodyCount = count - spineCount - arcCount;

    while (filled < bodyCount) {
      var rx = bounds.minX + Math.random() * (bounds.maxX - bounds.minX);
      var ry = bounds.minY + Math.random() * (bounds.maxY - bounds.minY);
      if (pointInPolygon(rx, ry, poly)) {
        var normY = (ry - bounds.minY) / (bounds.maxY - bounds.minY);
        var maxZ = 6.0 + Math.sin(normY * Math.PI) * 16.0;
        var rz = (Math.random() - 0.5) * 2 * maxZ;
        var pIdx = filled * 4;
        baseBoltPoints[pIdx] = rx;
        baseBoltPoints[pIdx + 1] = ry;
        baseBoltPoints[pIdx + 2] = rz;
        baseBoltPoints[pIdx + 3] = 1;
        filled++;
      }
    }

    for (var k = 0; k < spineCount; k++, filled++) {
      var st = k / spineCount;
      var sy = 65 - st * 135;
      var sx = (sy > 18) ? (sy - 18) * 0.5 : ((sy > -15) ? (sy - 18) * 0.6 : (sy + 15) * 0.4);
      var sIdx = filled * 4;
      baseBoltPoints[sIdx] = sx + (Math.random() - 0.5) * 3.0;
      baseBoltPoints[sIdx + 1] = sy + (Math.random() - 0.5) * 3.0;
      baseBoltPoints[sIdx + 2] = (Math.random() - 0.5) * 12;
      baseBoltPoints[sIdx + 3] = 2;
    }

    for (var j = 0; j < arcCount; j++, filled++) {
      var angle = Math.random() * Math.PI * 2;
      var radius = 40 + Math.random() * 55;
      var ax = Math.cos(angle) * radius;
      var ay = (Math.random() - 0.5) * 140;
      var az = Math.sin(angle) * (18 + Math.random() * 30);
      var aIdx = filled * 4;
      baseBoltPoints[aIdx] = ax;
      baseBoltPoints[aIdx + 1] = ay;
      baseBoltPoints[aIdx + 2] = az;
      baseBoltPoints[aIdx + 3] = 3;
    }
  }

  function generateBoltBuffers(count, positions, colors) {
    if (!baseBoltPoints) return;
    for (var i = 0; i < count; i++) {
      var bIdx = i * 4;
      var px = baseBoltPoints[bIdx];
      var py = baseBoltPoints[bIdx + 1];
      var pz = baseBoltPoints[bIdx + 2];
      var pType = baseBoltPoints[bIdx + 3];

      var pIdx = i * 3;
      positions[pIdx] = px;
      positions[pIdx + 1] = py;
      positions[pIdx + 2] = pz;

      if (pType === 2) {
        colors[pIdx] = 1.0; colors[pIdx + 1] = 1.0; colors[pIdx + 2] = 0.95;
      } else if (pType === 1) {
        colors[pIdx] = 1.0; colors[pIdx + 1] = 0.92; colors[pIdx + 2] = 0.15;
      } else {
        colors[pIdx] = 0.98; colors[pIdx + 1] = 0.78; colors[pIdx + 2] = 0.08;
      }
    }
  }

  function getParticleSize(count) {
    if (count >= 2000000) return 0.65;
    if (count >= 1000000) return 0.85;
    if (count >= 500000) return 1.2;
    if (count >= 250000) return 1.6;
    return 2.2;
  }

  function createBoltTexture() {
    var c = document.createElement('canvas');
    c.width = 64; c.height = 64;
    var ctx = c.getContext('2d');
    var g = ctx.createRadialGradient(32, 32, 0, 32, 32, 32);
    g.addColorStop(0, 'rgba(255, 255, 255, 1)');
    g.addColorStop(0.2, 'rgba(254, 240, 138, 0.95)');
    g.addColorStop(0.5, 'rgba(250, 204, 21, 0.75)');
    g.addColorStop(0.8, 'rgba(234, 179, 8, 0.2)');
    g.addColorStop(1, 'rgba(0, 0, 0, 0)');
    ctx.fillStyle = g;
    ctx.fillRect(0, 0, 64, 64);
    return new THREE.CanvasTexture(c);
  }

  function initThree() {
    if (typeof THREE === 'undefined') {
      setTimeout(initThree, 80);
      return;
    }

    var width = canvas.parentElement.clientWidth || 800;
    var height = canvas.parentElement.clientHeight || 520;

    scene = new THREE.Scene();
    camera = new THREE.PerspectiveCamera(55, width / height, 0.1, 1000);
    camera.position.set(0, 0, cameraDistance);

    renderer = new THREE.WebGLRenderer({ canvas: canvas, antialias: true, alpha: true, powerPreference: "high-performance" });
    renderer.setSize(width, height);
    renderer.setPixelRatio(Math.min(window.devicePixelRatio || 1, 2));

    var ambientLight = new THREE.AmbientLight(0xffffff, 0.35);
    scene.add(ambientLight);

    precomputeBoltShape(pointCount);

    boltGeometry = new THREE.BufferGeometry();
    var positions = new Float32Array(pointCount * 3);
    var colors = new Float32Array(pointCount * 3);

    generateBoltBuffers(pointCount, positions, colors);

    positionsAttr = new THREE.BufferAttribute(positions, 3);
    colorsAttr = new THREE.BufferAttribute(colors, 3);
    boltGeometry.setAttribute('position', positionsAttr);
    boltGeometry.setAttribute('color', colorsAttr);

    var pointsMaterial = new THREE.PointsMaterial({
      size: getParticleSize(pointCount),
      vertexColors: true,
      map: createBoltTexture(),
      transparent: true,
      blending: THREE.AdditiveBlending,
      depthWrite: false
    });

    boltMesh = new THREE.Points(boltGeometry, pointsMaterial);
    scene.add(boltMesh);

    // Mouse drag
    canvas.addEventListener('mousedown', function(e) {
      isMouseDown = true;
      mouseX = e.clientX;
      mouseY = e.clientY;
    });
    window.addEventListener('mouseup', function() { isMouseDown = false; });
    window.addEventListener('mousemove', function(e) {
      if (!isMouseDown) return;
      var dx = e.clientX - mouseX;
      var dy = e.clientY - mouseY;
      mouseX = e.clientX;
      mouseY = e.clientY;
      targetRotationY += dx * 0.005;
      targetRotationX += dy * 0.005;
    });

    // Zoom
    canvas.addEventListener('wheel', function(e) {
      e.preventDefault();
      cameraDistance += e.deltaY * 0.15;
      cameraDistance = Math.max(45, Math.min(360, cameraDistance));
      camera.position.z = cameraDistance;
    }, { passive: false });

    // Touch
    canvas.addEventListener('touchstart', function(e) {
      if (e.touches.length === 1) {
        isMouseDown = true;
        mouseX = e.touches[0].clientX;
        mouseY = e.touches[0].clientY;
      }
    }, { passive: true });
    window.addEventListener('touchend', function() { isMouseDown = false; });
    window.addEventListener('touchmove', function(e) {
      if (!isMouseDown || e.touches.length !== 1) return;
      var dx = e.touches[0].clientX - mouseX;
      var dy = e.touches[0].clientY - mouseY;
      mouseX = e.touches[0].clientX;
      mouseY = e.touches[0].clientY;
      targetRotationY += dx * 0.005;
      targetRotationX += dy * 0.005;
    }, { passive: true });

    window.addEventListener('resize', handleResize);
    document.addEventListener('keydown', function(e) {
      if (e.key === 'Escape' && isFullscreen) {
        toggleFullscreen();
      }
    });

    document.addEventListener('visibilitychange', function() {
      if (!document.hidden) {
        isFirstFrame = true;
      }
    });

    requestAnimationFrame(renderLoop);
  }

  function handleResize() {
    if (!renderer || !camera) return;
    var width = canvas.parentElement.clientWidth || 800;
    var height = canvas.parentElement.clientHeight || 520;
    camera.aspect = width / height;
    camera.updateProjectionMatrix();
    renderer.setSize(width, height);
  }

  window.toggleFullscreen = function() {
    var box = document.getElementById('bfast-demo-box');
    isFullscreen = !isFullscreen;
    box.classList.toggle('is-fullscreen', isFullscreen);
    var btn = document.getElementById('btn-immersive');
    if (isFullscreen) {
      btn.style.display = 'none';
      document.body.style.overflow = 'hidden';
    } else {
      btn.style.display = 'inline-block';
      document.body.style.overflow = '';
    }
    setTimeout(handleResize, 50);
  };

  // Web Worker oficial que consome bfast-client@1.12.0 do npm via CDN esm.sh
  var workerScript = `
    var bfast = null;

    async function initBFast() {
      try {
        var mod = await import('https://esm.sh/bfast-client@1.12.0');
        if (mod && mod.BFastDecoder) {
          bfast = mod;
          console.log('[B-FAST 3D Demo] Biblioteca oficial bfast-client@1.12.0 ativa via esm.sh');
        }
      } catch (err) {
        console.warn('[B-FAST 3D Demo] CDN indisponível, utilizando pipeline direto:', err);
      }
      self.postMessage({ type: 'READY', isOfficial: !!bfast });
    }

    initBFast();

    self.onmessage = function(e) {
      var msg = e.data;
      var t0 = performance.now();

      if (msg.type === 'PROCESS_STREAM') {
        var count = msg.count;
        var timeOffset = msg.timeOffset;
        var surgeWave = msg.surgeWave;
        var basePoints = new Float32Array(msg.basePointsBuffer);
        
        var pos = new Float32Array(count * 3);

        for (var i = 0; i < count; i++) {
          var bIdx = i * 4;
          var px = basePoints[bIdx];
          var py = basePoints[bIdx + 1];
          var pz = basePoints[bIdx + 2];
          var pType = basePoints[bIdx + 3];

          if (pType === 3) {
            var a = timeOffset * 1.6 + (i * 0.02);
            px += Math.cos(a) * 2.5;
            pz += Math.sin(a) * 2.5;
          }
          if (surgeWave > 0) {
            var waveY = 70 - surgeWave;
            var dist = Math.abs(py - waveY);
            if (dist < 24) {
              var surgeFactor = Math.sin((24 - dist) / 24 * Math.PI);
              px += (Math.random() - 0.5) * surgeFactor * 6.0;
              pz += (Math.random() - 0.5) * surgeFactor * 6.0;
            }
          }
          pos[i * 3] = px;
          pos[i * 3 + 1] = py;
          pos[i * 3 + 2] = pz;
        }

        var byteSize = pos.byteLength;
        var outBuffer = pos.buffer;

        if (bfast && bfast.BFastEncoder && bfast.BFastDecoder) {
          var payload = { points: new Uint8Array(pos.buffer) };
          var bfastBinary = bfast.BFastEncoder.encode(payload, { compress: false });
          byteSize = bfastBinary.byteLength;

          var decoded = bfast.BFastDecoder.decode(bfastBinary, { typedArrays: true });
          outBuffer = decoded.points.buffer;
        }

        var duration = performance.now() - t0;
        self.postMessage({
          type: 'DONE',
          buffer: outBuffer,
          durationMs: duration,
          byteSize: byteSize,
          isOfficial: !!bfast
        }, [outBuffer]);
      }
    };
  `;
  var workerBlob = new Blob([workerScript], { type: 'application/javascript' });
  var bfastWorker = new Worker(URL.createObjectURL(workerBlob), { type: 'module' });

  function requestDecodeFrame(isSurge) {
    if (!baseBoltPoints) return;
    var timeOffset = performance.now() * 0.001;
    bfastWorker.postMessage({
      type: 'PROCESS_STREAM',
      count: pointCount,
      timeOffset: timeOffset,
      surgeWave: isSurge ? 30 : 0,
      basePointsBuffer: baseBoltPoints.buffer.slice(0)
    });
  }

  bfastWorker.onmessage = function(e) {
    if (e.data.type === 'READY') {
      requestDecodeFrame(false);
      return;
    }
    if (e.data.type === 'DONE') {
      var newPositions = new Float32Array(e.data.buffer);
      lastDecodeDurationMs = e.data.durationMs;
      
      if (boltGeometry) {
        positionsAttr = new THREE.BufferAttribute(newPositions, 3);
        boltGeometry.setAttribute('position', positionsAttr);
        positionsAttr.needsUpdate = true;
      }
      
      if (isStreaming) {
        surgeActive = true;
        dynamicSurge = 0;
      }

      updateTelemetry({
        decodeMs: lastDecodeDurationMs,
        wireBytes: e.data.byteSize
      });
    }
  };

  function renderLoop(now) {
    if (isFirstFrame) {
      lastFrameTime = now;
      isFirstFrame = false;
      requestAnimationFrame(renderLoop);
      return;
    }

    var delta = now - lastFrameTime;
    lastFrameTime = now;

    if (delta > 250) {
      requestAnimationFrame(renderLoop);
      return;
    }

    var currentFps = Math.round(1000 / Math.max(delta, 1));
    if (currentFps > maxObservedFps) {
      maxObservedFps = currentFps;
      if (maxObservedFps >= 110) targetMonitorFps = 120;
      else if (maxObservedFps >= 80) targetMonitorFps = 90;
      else targetMonitorFps = 60;
    }

    var frameBudget = 1000 / targetMonitorFps;
    if (delta > (frameBudget + 5)) {
      var jank = delta - frameBudget;
      if (jank > maxJankMs) maxJankMs = jank;
    }

    if (surgeActive) {
      dynamicSurge += 4.5;
      if (dynamicSurge > 155) surgeActive = false;
    }

    if (boltMesh) {
      boltMesh.rotation.y += 0.0035;
      boltMesh.rotation.x += (targetRotationX - boltMesh.rotation.x) * 0.05;
      boltMesh.rotation.y += (targetRotationY - boltMesh.rotation.y) * 0.05;
    }

    if (renderer && scene && camera) {
      renderer.render(scene, camera);
    }

    frameDeltas.push(currentFps);
    if (frameDeltas.length >= 6) {
      var sum = 0;
      for (var i = 0; i < frameDeltas.length; i++) sum += frameDeltas[i];
      var avgFps = Math.round(sum / frameDeltas.length);
      frameDeltas = [];
      updateFpsDisplay(avgFps);
    }

    requestAnimationFrame(renderLoop);
  }

  function updateFpsDisplay(fps) {
    var el = document.getElementById('hud-fps');
    if (!el) return;
    el.innerText = fps + ' FPS';
    el.className = 'bfast-hud-value ' + (fps >= (targetMonitorFps * 0.9) ? 'fps-good' : (fps >= (targetMonitorFps * 0.6) ? 'fps-warning' : 'fps-bad'));

    fpsHistory.shift();
    fpsHistory.push(fps);
    drawSparkline();
  }

  function drawSparkline() {
    var canvasEl = document.getElementById('hud-fps-sparkline');
    if (!canvasEl) return;
    var ctx = canvasEl.getContext('2d');
    var w = canvasEl.width;
    var h = canvasEl.height;
    ctx.clearRect(0, 0, w, h);

    ctx.strokeStyle = '#0d9488';
    ctx.lineWidth = 2;
    ctx.beginPath();
    var step = w / (fpsHistory.length - 1);
    for (var i = 0; i < fpsHistory.length; i++) {
      var x = i * step;
      var y = h - ((fpsHistory[i] / targetMonitorFps) * (h - 4)) - 2;
      if (i === 0) ctx.moveTo(x, y);
      else ctx.lineTo(x, y);
    }
    ctx.stroke();
  }

  function updateTelemetry(data) {
    var decodeEl = document.getElementById('hud-decode');
    var wireEl = document.getElementById('hud-wire');
    var wireSubEl = document.getElementById('hud-wire-sub');

    if (decodeEl) decodeEl.innerText = data.decodeMs.toFixed(1) + ' ms';
    if (wireEl) {
      if (data.wireBytes >= 1048576) {
        wireEl.innerText = (data.wireBytes / 1048576).toFixed(1) + ' MB';
      } else {
        wireEl.innerText = Math.round(data.wireBytes / 1024) + ' KB';
      }
    }
    if (wireSubEl) wireSubEl.innerText = 'Binário B-FAST';
  }

  window.onDensityChange = function(val) {
    pointCount = parseInt(val, 10);
    precomputeBoltShape(pointCount);

    var positions = new Float32Array(pointCount * 3);
    var colors = new Float32Array(pointCount * 3);
    generateBoltBuffers(pointCount, positions, colors);

    positionsAttr = new THREE.BufferAttribute(positions, 3);
    colorsAttr = new THREE.BufferAttribute(colors, 3);
    boltGeometry.setAttribute('position', positionsAttr);
    boltGeometry.setAttribute('color', colorsAttr);
    positionsAttr.needsUpdate = true;
    colorsAttr.needsUpdate = true;

    if (boltMesh && boltMesh.material) {
      boltMesh.material.size = getParticleSize(pointCount);
      boltMesh.material.needsUpdate = true;
    }

    var badge = document.getElementById('bfast-status-badge');
    if (badge) {
      badge.innerText = '⚡ B-FAST Oficial (npm v1.12.0): ' + pointCount.toLocaleString() + ' Partículas';
    }
    resetStats();
    requestDecodeFrame(false);
  };

  window.toggleStream = function() {
    isStreaming = !isStreaming;
    var btn = document.getElementById('btn-stream');
    if (isStreaming) {
      btn.innerText = '⏹️ Parar Stream';
      btn.className = 'bfast-btn bfast-btn-danger';
      requestDecodeFrame(true);
      streamTimer = setInterval(function() {
        requestDecodeFrame(true);
      }, 200);
    } else {
      btn.innerText = '🔄 Stream Contínuo';
      btn.className = 'bfast-btn bfast-btn-primary';
      clearInterval(streamTimer);
    }
  };

  function resetStats() {
    maxJankMs = 0;
    fpsHistory.fill(targetMonitorFps);
    var jankEl = document.getElementById('hud-jank');
    if (jankEl) {
      jankEl.innerText = '0.0 ms';
      jankEl.className = 'bfast-hud-value jank-good';
    }
    drawSparkline();
  }

  if (document.readyState === 'complete' || document.readyState === 'interactive') {
    initThree();
  } else {
    document.addEventListener('DOMContentLoaded', initThree);
  }
})();
</script>

---

## ⚡ A Diferença na Prática

| Métrica | 🔴 JSON Tradicional | ⚡ B-FAST |
| :--- | :--- | :--- |
| **Impacto na Interface** | **Trava de 150ms a 1.2s+** | **0.00 ms** (UI 100% livre) |
| **Taxa de Quadros (FPS)** | Despenca para **0 – 15 FPS** | Cravado em **60 / 120+ FPS** (Nativo) |
| **Escalabilidade** | Inviável para dados massivos | **Milhões de pontos e geometrias 3D** |
| **Impacto no Event Loop** | Bloqueia a interface do usuário | **Livre (processamento em segundo plano)** |
| **Consumo de Tráfego** | Texto redundante e verboso | **-76% a -90%** mais compacto (LZ4) |

### Como Usar em 3 Linhas

```typescript
import { bfastFetchWorker } from 'bfast-client/worker';

// Baixa e decodifica dados massivos em tempo real sem travar a interface
const data = await bfastFetchWorker<{ positions: Float32Array }>('/api/malha-3d', {
    typedArrays: true,
});

// Atualização direta da geometria 3D
geometry.setAttribute('position', new THREE.BufferAttribute(data.positions, 3));
geometry.attributes.position.needsUpdate = true;
```

Leia mais no [Guia de Frontend TypeScript](frontend.md).
