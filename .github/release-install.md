

## Instalación

1. Descarga `Actions-Lights_{{VERSION}}_universal.dmg` (Apple Silicon e Intel) y arrastra la app a **Aplicaciones**.
2. La app no está firmada con un certificado de Apple. La primera vez, ábrela con **clic derecho → Abrir**, o ejecuta:

   ```sh
   xattr -cr "/Applications/Actions Lights.app"
   ```

3. Si tienes `gh` con sesión iniciada no hay nada que configurar. Si no, conecta tu cuenta en **Ajustes → Cuenta**.

Requiere macOS 10.13 o posterior.

### Linux (x86_64)

- **Debian/Ubuntu:** `sudo apt install ./Actions.Lights_{{VERSION}}_amd64.deb`
- **Otras distribuciones:** descarga `Actions.Lights_{{VERSION}}_amd64.AppImage`, dale permiso de ejecución (`chmod +x`) y ábrelo.

El estado se abre desde el menú del icono (**Ver estado**). En GNOME hace falta la extensión *AppIndicator and KStatusNotifierItem Support* para ver el icono.

### Windows (x86_64)

Descarga `Actions.Lights_{{VERSION}}_x64-setup.exe` y ejecútalo; se instala para tu usuario, sin permisos de administrador. El instalador no está firmado, así que SmartScreen avisa la primera vez: pulsa **Más información → Ejecutar de todas formas**. El estado se abre con clic en el icono de la bandeja (puede estar dentro de la flecha **^** de la barra de tareas).
