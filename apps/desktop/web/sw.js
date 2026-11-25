// Service worker de la versión web: con internet baja todo de la red y guarda una copia;
// sin internet sirve la copia, así la app instalada abre también sin conexión.
// ponytail: las versiones viejas (trunk les pone hash al nombre) se quedan en la caché;
// borrarlas al activar si el espacio llega a importar.
const CACHE = "logicsm";

self.addEventListener("install", () => self.skipWaiting());
self.addEventListener("activate", (e) => e.waitUntil(self.clients.claim()));

self.addEventListener("fetch", (e) => {
  if (e.request.method !== "GET" || !e.request.url.startsWith(self.location.origin)) return;
  e.respondWith(
    fetch(e.request)
      .then((respuesta) => {
        if (respuesta.ok) {
          const copia = respuesta.clone();
          e.waitUntil(caches.open(CACHE).then((c) => c.put(e.request, copia)));
        }
        return respuesta;
      })
      .catch(() => caches.match(e.request).then((r) => r ?? Response.error())),
  );
});
