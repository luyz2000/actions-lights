# Changelog

## [1.3.0](https://github.com/luyz2000/actions-lights/compare/v1.2.0...v1.3.0) (2026-10-30)


### Novedades

* in-app updates from GitHub releases with signed updater bundles ([094eedc](https://github.com/luyz2000/actions-lights/commit/094eedc85a9b52aef006f9d6f06b897f01dfd9fb))

## [1.2.0](https://github.com/luyz2000/actions-lights/compare/v1.1.1...v1.2.0) (2026-10-30)


### Novedades

* idle state shows the app logo in the menu bar with an all-clear summary ([20b2366](https://github.com/luyz2000/actions-lights/commit/20b236697fc3572130ed44d0d57e0b4d163a7460))

## [1.1.1](https://github.com/luyz2000/actions-lights/compare/v1.1.0...v1.1.1) (2026-10-30)


### Correcciones

* show the popover over fullscreen apps as a non-activating panel and keep settings reachable via Dock and Cmd+Tab ([b4b1c1c](https://github.com/luyz2000/actions-lights/commit/b4b1c1ca8d2d639c856e1f80d5b9fc573aac3355))

## [1.1.0](https://github.com/luyz2000/actions-lights/compare/v1.0.0...v1.1.0) (2026-10-30)


### Novedades

* descriptive menu-bar text, clearer notifications and optional sounds ([cd84d67](https://github.com/luyz2000/actions-lights/commit/cd84d675537e08810caaf8c081f9afbf77350336))
* watch the latest run of any branch or event and show where it came from ([d0443d6](https://github.com/luyz2000/actions-lights/commit/d0443d6a90019fd220f3e6d2a2f57b79fe9e6c5d))


### Correcciones

* popover card shows the full workflow name, a real gear icon and the new logo in settings ([7134efb](https://github.com/luyz2000/actions-lights/commit/7134efb43f944adca6e6603f98e3716d66f8a6f8))
* register macOS login item via SMAppService so it shows the app name and icon ([0c4be61](https://github.com/luyz2000/actions-lights/commit/0c4be6155ce8b910624c9e78321ac495aeda9744))
* show the tray popover over fullscreen apps and allow a single running instance ([7350182](https://github.com/luyz2000/actions-lights/commit/735018264cd8ab0a02b00e48f4c6c069367fd131))

## [1.0.0](https://github.com/luyz2000/actions-lights/releases/tag/v1.0.0) (2026-10-27)

### Novedades

* Semáforo en la barra de menús con anillo de progreso mientras un workflow corre.
* Panel con los workflows por repo: paso en curso, tiempo frente al habitual y paso donde falló.
* Notificaciones al terminar bien o al fallar, configurables por workflow.
* Elegir repos desde la lista, por `owner/repo` o desde una carpeta local; rama y evento por workflow.
* Cuenta detectada en orden: GitHub CLI, token personal o Iniciar sesión con GitHub.
* Consultas adaptativas que cuidan el límite de la API y se recuperan tras suspender el Mac.
