# Cómo contribuir a Logic_F-4:13

¡Gracias por tu interés! Esta guía explica cómo proponer cambios.

Al participar aceptas el [Código de Conducta](CODE_OF_CONDUCT.md).

## Antes de empezar

- **Bugs:** abre un issue con la plantilla *Reporte de bug*. Incluye el código `.SA` mínimo que reproduce el problema.
- **Ideas nuevas:** abre primero un issue con la plantilla *Propuesta*. Así se discute antes de que inviertas tiempo en un PR que quizá no encaje con el diseño (ver el [diseño](docs/DISENO.md)).
- **Vulnerabilidades de seguridad:** no abras un issue público; sigue [SECURITY.md](SECURITY.md).

## Flujo de trabajo

La rama `main` está protegida: todo cambio entra por Pull Request.

1. Haz un fork del repositorio y crea una rama desde `main`:
   ```sh
   git switch -c feat/descripcion-corta
   ```
   Prefijos de rama: `feat/`, `fix/`, `docs/`, `refactor/`, `test/`, `ci/`.
2. Haz tus cambios con commits pequeños y claros (ver abajo).
3. Verifica localmente antes de abrir el PR:
   ```sh
   cargo fmt --all
   cargo clippy --workspace --all-targets -- -D warnings
   cargo test --workspace
   ```
4. Abre el PR contra `main` y completa la plantilla.
5. El mantenedor revisa el PR. Los PRs se integran con *squash merge*.

## Mensajes de commit

Se usa [Conventional Commits](https://www.conventionalcommits.org/es/):

```
<tipo>(<alcance opcional>): <descripción en imperativo>
```

| Tipo | Uso |
|---|---|
| `feat` | Funcionalidad nueva |
| `fix` | Corrección de un bug |
| `docs` | Solo documentación |
| `refactor` | Cambio de código que no altera el comportamiento |
| `test` | Agregar o corregir tests |
| `ci` | GitHub Actions y automatización |
| `chore` | Mantenimiento (dependencias, configuración) |

Ejemplos: `feat(parser): soportar arreglos de instancias`, `fix(sim): estabilizar realimentación de NOR`.

## Estilo de código

- Rust estable, formateado con `rustfmt` y sin advertencias de `clippy`.
- Todo cambio en el compilador o el simulador debe venir con tests.
- Mensajes de error y documentación para el usuario: en español.

## Licencia de las contribuciones

Salvo que indiques lo contrario, toda contribución que envíes para su inclusión en el proyecto se licencia bajo [MIT](LICENSE-MIT) o [Apache-2.0](LICENSE-APACHE), a elección del usuario, sin condiciones adicionales.
