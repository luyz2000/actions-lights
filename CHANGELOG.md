# Changelog

## [1.8.0](https://github.com/luyz2000/actions-lights/compare/v1.7.0...v1.8.0) (2026-10-31)


### Novedades

* run on Linux (deb and AppImage), opening the status panel from the tray menu ([79ff5a3](https://github.com/luyz2000/actions-lights/commit/79ff5a3fc66e953eb482b5220d2fbc84e366324c))

## [1.7.0](https://github.com/luyz2000/actions-lights/compare/v1.6.0...v1.7.0) (2026-10-31)


### Novedades

* choose the app language in Settings › General, or keep following the system ([94d234c](https://github.com/luyz2000/actions-lights/commit/94d234ce3cebbd128efbafc3dbad59c2b669db11))

## [1.6.0](https://github.com/luyz2000/actions-lights/compare/v1.5.0...v1.6.0) (2026-10-31)


### Novedades

* show the app in the system language (English, Spanish, Russian, French, Japanese and Chinese), falling back to English ([fe0dfc7](https://github.com/luyz2000/actions-lights/commit/fe0dfc79922c6aee18b052ce775519af80e76c97))

## [1.5.0](https://github.com/luyz2000/actions-lights/compare/v1.4.0...v1.5.0) (2026-10-30)


### Novedades

* show the installed version and a check-for-updates button in Settings › General ([05f64d5](https://github.com/luyz2000/actions-lights/commit/05f64d5e2cc83db10b048284c37432966c1cf46c))

## [1.4.0](https://github.com/luyz2000/actions-lights/compare/v1.3.0...v1.4.0) (2026-10-30)


### Novedades

* native macOS notifications with click-to-open the run, paused until the app is signed by Apple ([9d6a81c](https://github.com/luyz2000/actions-lights/commit/9d6a81c4492ddbf11cc4708d80290890dbdb4f8c))
* show the PR title or commit and who triggered each run ([66e3731](https://github.com/luyz2000/actions-lights/commit/66e3731f7f0cdd719047703d147fa12d428e256a))


### Correcciones

* keep notifications off with a coming-soon notice until the app is signed by Apple ([a3e7c48](https://github.com/luyz2000/actions-lights/commit/a3e7c4840b8274071976e584e082a3ccdb42ba8b))

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
