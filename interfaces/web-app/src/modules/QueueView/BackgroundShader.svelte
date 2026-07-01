<script>
  import { onMount, onDestroy } from "svelte";
  import { library } from "../../library.svelte.js";
  
  import vertexShaderSource from "./Shaders/Quad.vert?raw";
  import internalFragmentShader from "./Shaders/Simplex.frag?raw";

  let { colors =[], coverSize = 0, visible = false, isPlaying = false } = $props();

  const PALETTE_SIZE_LIMIT = 12;

  let canvasEl;
  let gl;
  let program;
  let animationFrame;
  
  let totalTime = 0;
  let lastFrameTime = 0;
  let isTabVisible = $state(true);
  let randomOffset = Math.random() * 1000.0;

  const floatColorsOklab = new Float32Array(24 * 3);
  const floatRatios = new Float32Array(24);
  let activeColorCount = 0;
  const DEFAULT_PALETTE = ["#242424"];

  let needsRedraw = true;
  let shaderSource = $state(internalFragmentShader);

  function hexToOklab(hex) {
    if (hex.startsWith('#')) hex = hex.slice(1);
    if (hex.length === 3) hex = hex.split('').map(c => c + c).join('');
    
    const r = parseInt(hex.slice(0, 2), 16) / 255.0;
    const g = parseInt(hex.slice(2, 4), 16) / 255.0;
    const b = parseInt(hex.slice(4, 6), 16) / 255.0;
    
    const lin = c => c >= 0.04045 ? Math.pow((c + 0.055) / 1.055, 2.4) : c / 12.92;
    const lr = lin(r);
    const lg = lin(g);
    const lb = lin(b);
    
    const l = 0.4122214708 * lr + 0.5363325363 * lg + 0.0514459929 * lb;
    const m = 0.2119034982 * lr + 0.6806995451 * lg + 0.1073969566 * lb;
    const s = 0.0883024619 * lr + 0.2817188376 * lg + 0.6299787005 * lb;
    
    const l_ = Math.cbrt(l);
    const m_ = Math.cbrt(m);
    const s_ = Math.cbrt(s);
    
    const L = 0.2104542553 * l_ + 0.7936177850 * m_ - 0.0040720468 * s_;
    const A = 1.9779984951 * l_ - 2.4285922050 * m_ + 0.4505937099 * s_;
    const B = 0.0259040371 * l_ + 0.7827717662 * m_ - 0.8086757660 * s_;
    
    return [L, A, B];
  }

  function parseOklch(str) {
    const m = str.match(/oklch\(([\d.]+)%\s+([\d.]+)\s+([\d.]+)\)/);
    return m ? { L: parseFloat(m[1]), C: parseFloat(m[2]), H: parseFloat(m[3]) } : { L: 0, C: 0, H: 0 };
  }

  function getChroma(c) {
    if (Array.isArray(c) && c.length > 1) {
      return parseOklch(c[1]).C;
    }
    return 0;
  }

  function shuffle(array) {
    for (let i = array.length - 1; i > 0; i--) {
      const j = Math.floor(Math.random() * (i + 1));
      [array[i], array[j]] = [array[j], array[i]];
    }
    return array;
  }

  async function loadExternalShader(path) {
    if (!path) {
        shaderSource = internalFragmentShader;
        return;
    }
    try {
        const res = await fetch(`/api/interfaces/default/assets/${path}?v=${Date.now()}`);
        if (res.ok) {
            shaderSource = await res.text();
        } else {
            shaderSource = internalFragmentShader;
        }
    } catch (e) {
        shaderSource = internalFragmentShader;
    }
  }

  $effect(() => {
    loadExternalShader(library.config.theme?.shader?.path);
  });

  $effect(() => {
    let palette = (colors && colors.length > 0) ? [...colors] : [...DEFAULT_PALETTE];
    const order = library.config.theme?.shader?.order || "random";
    
    if (order !== "original") {
      palette.sort((a, b) => getChroma(b) - getChroma(a));
    }
    
    palette = palette.slice(0, PALETTE_SIZE_LIMIT);

    if (order === "random") {
      shuffle(palette);
    } else if (order === "ratio") {
      palette.sort((a, b) => {
        const rA = Array.isArray(a) ? parseFloat(a[a.length - 1]) : 0;
        const rB = Array.isArray(b) ? parseFloat(b[b.length - 1]) : 0;
        return rB - rA; 
      });
    } else if (order.startsWith("oklch,")) {
      const comp = order.split(",")[1];
      palette.sort((a, b) => {
        if (!Array.isArray(a) || !Array.isArray(b) || a.length < 2 || b.length < 2) return 0;
        const valA = parseOklch(a[1])[comp] || 0;
        const valB = parseOklch(b[1])[comp] || 0;
        return valA - valB;
      });
    }

    activeColorCount = palette.length;
    
    let hasRatios = false;
    for (let i = 0; i < activeColorCount; i++) {
      if (Array.isArray(palette[i]) && palette[i].length > 1) {
        hasRatios = true;
        break;
      }
    }

    let rawRatios = new Array(activeColorCount).fill(0);
    let totalRaw = 0;
    
    for (let i = 0; i < activeColorCount; i++) {
      const c = palette[i];
      if (hasRatios) {
        rawRatios[i] = Array.isArray(c) ? parseFloat(c[c.length - 1]) : 0.0;
      } else {
        rawRatios[i] = 1.0 / (i + 1.0);
      }
      totalRaw += rawRatios[i];
    }

    if (totalRaw > 0) {
      for (let i = 0; i < activeColorCount; i++) {
        rawRatios[i] /= totalRaw;
      }
    } else {
      for (let i = 0; i < activeColorCount; i++) {
        rawRatios[i] = 1.0 / activeColorCount;
      }
    }

    const equalize = library.config.theme?.shader?.equalize ?? 0;
    const avgRatio = 1.0 / activeColorCount;

    for (let i = 0; i < activeColorCount; i++) {
      rawRatios[i] = (rawRatios[i] * (1.0 - equalize)) + (avgRatio * equalize);
    }

    for (let i = 0; i < 24; i++) {
      if (i < activeColorCount) {
        const c = palette[i];
        const hex = Array.isArray(c) ? c[0] : (c.hex || c);
        const [L, a, b] = hexToOklab(hex);
        
        floatColorsOklab[i * 3 + 0] = L;
        floatColorsOklab[i * 3 + 1] = a;
        floatColorsOklab[i * 3 + 2] = b;
        floatRatios[i] = rawRatios[i];
      } else {
        floatColorsOklab[i * 3 + 0] = 0.0;
        floatColorsOklab[i * 3 + 1] = 0.0;
        floatColorsOklab[i * 3 + 2] = 0.0;
        floatRatios[i] = 0.0;
      }
    }
    needsRedraw = true;
  });

  function createShader(gl, type, source) {
    const shader = gl.createShader(type);
    gl.shaderSource(shader, source);
    gl.compileShader(shader);
    if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) {
      gl.deleteShader(shader);
      return null;
    }
    return shader;
  }

  function initGL() {
    if (!canvasEl) return;
    gl = canvasEl.getContext("webgl2", { 
      alpha: false, 
      antialias: true,
      premultipliedAlpha: false,
      preserveDrawingBuffer: false
    });
    
    if (!gl) return;

    const vs = createShader(gl, gl.VERTEX_SHADER, vertexShaderSource);
    const fs = createShader(gl, gl.FRAGMENT_SHADER, shaderSource);

    if (!vs || !fs) return;

    if (program) gl.deleteProgram(program);
    program = gl.createProgram();
    gl.attachShader(program, vs);
    gl.attachShader(program, fs);
    gl.linkProgram(program);

    const vertices = new Float32Array([-1, -1, 1, -1, -1, 1, -1, 1, 1, -1, 1, 1]);
    const buffer = gl.createBuffer();
    gl.bindBuffer(gl.ARRAY_BUFFER, buffer);
    gl.bufferData(gl.ARRAY_BUFFER, vertices, gl.STATIC_DRAW);

    const positionLoc = gl.getAttribLocation(program, "position");
    gl.enableVertexAttribArray(positionLoc);
    gl.vertexAttribPointer(positionLoc, 2, gl.FLOAT, false, 0, 0);

    lastFrameTime = performance.now();
    startLoop();
  }

  $effect(() => {
    if (shaderSource) initGL();
  });

  function startLoop() {
    if (animationFrame) cancelAnimationFrame(animationFrame);
    lastFrameTime = performance.now();
    render();
  }

  function render() {
    if (!gl || !program) return;

    if (!visible || !isTabVisible || !library.isShaderActive) {
      animationFrame = requestAnimationFrame(render);
      return;
    }

    const now = performance.now();
    let timeAdvanced = false;

    if (isPlaying) {
      let delta = (now - lastFrameTime) / 1000;
      if (delta > 0.1) delta = 0.016;
      totalTime += delta;
      timeAdvanced = true;
    }
    lastFrameTime = now;

    if (timeAdvanced || needsRedraw) {
      gl.viewport(0, 0, canvasEl.width, canvasEl.height);
      gl.useProgram(program);

      gl.uniform1f(gl.getUniformLocation(program, "iTime"), totalTime);
      gl.uniform1f(gl.getUniformLocation(program, "iRandom"), randomOffset);
      gl.uniform2f(gl.getUniformLocation(program, "iResolution"), canvasEl.width, canvasEl.height);
      
      const dpr = window.devicePixelRatio || 1;
      gl.uniform1f(gl.getUniformLocation(program, "iCoverSize"), coverSize * dpr);

      gl.uniform3fv(gl.getUniformLocation(program, "iColorsOklab"), floatColorsOklab);
      gl.uniform1fv(gl.getUniformLocation(program, "iRatios"), floatRatios);
      gl.uniform1i(gl.getUniformLocation(program, "iCount"), activeColorCount);

      const s = library.config.theme?.shader || {};
      gl.uniform1f(gl.getUniformLocation(program, "iSpeed"), s.speed ?? 0.007);
      gl.uniform1f(gl.getUniformLocation(program, "iZoom"), s.zoom ?? 0.4);
      gl.uniform1f(gl.getUniformLocation(program, "iBlur"), s.blur ?? 0.8);

      gl.uniform1f(gl.getUniformLocation(program, "iGrain"), s.grain ?? 0.01);
      gl.uniform1f(gl.getUniformLocation(program, "iEqualize"), s.equalize ?? 1.0);

      gl.drawArrays(gl.TRIANGLES, 0, 6);
      needsRedraw = false;
    }

    animationFrame = requestAnimationFrame(render);
  }

  function handleResize() {
    if (canvasEl) {
      const dpr = window.devicePixelRatio || 1;
      canvasEl.width = window.innerWidth * dpr;
      canvasEl.height = window.innerHeight * dpr;
      needsRedraw = true;
    }
  }

  function handleVisibilityChange() {
    isTabVisible = !document.hidden;
  }

  $effect(() => {
    if (colors || coverSize || library.config.theme?.shader) {
      handleResize();
    }
  });

  $effect(() => {
    if (visible && isTabVisible) {
      lastFrameTime = performance.now();
      needsRedraw = true;
    }
  });

  onMount(() => {
    handleResize();
    initGL();
    window.addEventListener("resize", handleResize);
    document.addEventListener("visibilitychange", handleVisibilityChange);
  });

  onDestroy(() => {
    if (animationFrame) cancelAnimationFrame(animationFrame);
    window.removeEventListener("resize", handleResize);
    document.removeEventListener("visibilitychange", handleVisibilityChange);
  });
</script>

<canvas
  bind:this={canvasEl}
  style="
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    z-index: 0;
    pointer-events: none;
    opacity: {library.isShaderActive && colors.length > 0 ? 1 : 0};
    transition: opacity 0.3s ease;
  "
></canvas>
