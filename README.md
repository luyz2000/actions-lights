# Actions Lights

El estado de tus workflows de GitHub Actions en la barra de menús de macOS. Mergeas a tu rama de release, sigues trabajando, y la barra te avisa cuando el deploy termina o falla.

- **Semáforo en la barra:** rojo si algo falló, amarillo mientras corre (con anillo de progreso), verde si todo terminó bien.
- **Panel al hacer clic:** workflows agrupados por repo, paso en curso, tiempo frente al habitual y el paso donde falló. Un clic abre el run.
- **Notificaciones** al terminar bien o al fallar, configurables por workflow.
- **Cuenta detectada sola:** GitHub CLI (`gh`), un token personal en el Llavero o "Iniciar sesión con GitHub".

## Instalación

Descarga el `.dmg` del [último release](https://github.com/luyz2000/actions-lights/releases/latest) y arrastra la app a **Aplicaciones**.

La app aún no está firmada con un certificado de Apple. La primera vez, ábrela con **clic derecho → Abrir**, o ejecuta:

```sh
xattr -cr "/Applications/Actions Lights.app"
```

En Windows (x86_64) ejecuta el `-setup.exe` del mismo release. No está firmado: si SmartScreen avisa, pulsa **Más información → Ejecutar de todas formas**.

Si tienes `gh` con sesión iniciada no hay nada que configurar. Si no, ve a **Ajustes → Cuenta**. Un token personal necesita el permiso `repo` (clásico) o *Actions: read* (fine-grained).
