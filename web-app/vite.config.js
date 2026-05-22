import { defineConfig } from 'vite'
import { svelte } from '@sveltejs/vite-plugin-svelte'
import path from 'path'
import net from 'node:net'
import tls from 'node:tls'
import { Stream } from 'node:stream'
import http from 'node:http'

const patch = function () { this.destroy(); };
[net.Socket, tls.TLSSocket, Stream].forEach((cls) => {
  if (cls && cls.prototype && !cls.prototype.destroySoon) {
    cls.prototype.destroySoon = patch;
  }
});

export default defineConfig({
  plugins: [svelte()],
  resolve: {
    alias: {
      '$modules': path.resolve(__dirname, './src/modules'),
    }
  },
  server: {
    port: 5173,
    proxy: {
      '/api': {
        target: 'http://127.0.0.1:8000',
        changeOrigin: true,
        secure: false,
        agent: new http.Agent({ keepAlive: true }),
      },
      '/ws': {
        target: 'ws://127.0.0.1:8000',
        ws: true
      }
    },
  },
})
