// Service worker de la versión web: con internet baja todo de la red y guarda una copia;
// sin internet sirve la copia, así la app instalada abre también sin conexión.
// ponytail: las versiones viejas (trunk les pone hash al nombre) se quedan en la caché;
// borrarlas al activar si el espacio llega a importar.
const CACHE = "logicsm";

// Al instalarse guarda ya la página y todo lo que enlaza (el .js y el .wasm con hash de
// trunk): la primera carga pasa antes de que el service worker controle la página, así que
// sin esto la app no abría sin internet hasta la segunda visita. En iPhone la app de la
// pantalla de inicio tiene su propia caché, aparte de Safari.
self.addEventListener("install", (e) => {
  self.skipWaiting();
  e.waitUntil(
    (async () => {
      const pagina = await fetch("./");
      const html = await pagina.clone().text();
      const enlaces = [...html.matchAll(/(?:href|src)="([^"]+)"/g)].map((m) => m[1]);
      const cache = await caches.open(CACHE);
      await cache.put("./", pagina);
      await cache.addAll(enlaces);
    })(),
  );
});
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
      // ignoreSearch: el ícono puede abrir la página con "?algo" al final
      .catch(() => caches.match(e.request, { ignoreSearch: true }).then((r) => r ?? Response.error())),
  );
});
