---
name: commit
description: Crear commits bien formateados con el formato de conventional commit en español.
---

# Smart Git Commit (Español)

Crear commit bien formateado: $ARGUMENTS

## Estado Actual del Repositorio

- Estado de Git: !`git status --porcelain`
- Rama actual: !`git branch --show-current`
- Cambios en staging: !`git diff --cached --stat`
- Cambios sin staging: !`git diff --stat`
- Commits recientes: !`git log --oneline -5`

## Qué Hace Este Comando

1. Verifica qué archivos están en staging con `git status`
2. Si hay 0 archivos en staging, automáticamente añade todos los archivos modificados y nuevos con `git add`
3. Ejecuta `git diff` para entender qué cambios se están commitando
4. Analiza el diff para determinar si hay múltiples cambios lógicos distintos
5. Si detecta múltiples cambios distintos, sugiere dividir el commit en varios commits más pequeños
6. Para cada commit (o el commit único si no se divide), crea un mensaje usando el formato conventional commit

## Mejores Prácticas para Commits

- Seguir la especificación de Conventional Commits como se describe a continuación
- **IMPORTANTE: Los mensajes de commit deben escribirse en español**
- El tipo (feat, fix, etc.) se mantiene en inglés según el estándar
- La descripción, cuerpo y pies de página deben estar en español

# Conventional Commits 1.0.0 (Adaptado al Español)

## Resumen

La especificación de Conventional Commits es una convención ligera sobre los mensajes de commit. Proporciona un conjunto fácil de reglas para crear un historial de commits explícito, lo que facilita la escritura de herramientas automatizadas. Esta convención encaja con [SemVer](http://semver.org), describiendo las características, correcciones y cambios importantes realizados en los mensajes de commit.

El mensaje de commit debe estructurarse de la siguiente manera:

```
<tipo>[ámbito opcional]: <descripción en español>

[cuerpo opcional en español]

[pie(s) de página opcional(es) en español]
```

El commit contiene los siguientes elementos estructurales para comunicar la intención:

1.  **fix:** un commit del _tipo_ `fix` corrige un error en el código (esto se correlaciona con [`PATCH`](http://semver.org/#summary) en Versionado Semántico).
2.  **feat:** un commit del _tipo_ `feat` introduce una nueva característica en el código (esto se correlaciona con [`MINOR`](http://semver.org/#summary) en Versionado Semántico).
3.  **BREAKING CHANGE:** un commit que tiene un pie de página `BREAKING CHANGE:`, o añade un `!` después del tipo/ámbito, introduce un cambio incompatible en la API (correlacionándose con [`MAJOR`](http://semver.org/#summary) en Versionado Semántico). Un BREAKING CHANGE puede ser parte de commits de cualquier _tipo_.
4.  Otros _tipos_ además de `fix:` y `feat:` están permitidos, por ejemplo [@commitlint/config-conventional](https://github.com/conventional-changelog/commitlint/tree/master/%40commitlint/config-conventional) recomienda `build:`, `chore:`, `ci:`, `docs:`, `style:`, `refactor:`, `perf:`, `test:`, y otros.
5.  Se pueden proporcionar _pies de página_ además de `BREAKING CHANGE: <descripción>` y seguir una convención similar al [formato de git trailer](https://git-scm.com/docs/git-interpret-trailers).

## Ejemplos en Español

### Commit con descripción y pie de página de cambio importante

```
feat: permitir que el objeto de configuración proporcionado extienda otras configuraciones

BREAKING CHANGE: la clave `extends` en el archivo de configuración ahora se usa para extender otros archivos de configuración
```

### Commit con `!` para llamar la atención sobre cambio importante

```
feat!: enviar un email al cliente cuando se envía un producto
```

### Commit con ámbito y `!` para llamar la atención sobre cambio importante

```
feat(api)!: enviar un email al cliente cuando se envía un producto
```

### Commit con `!` y pie de página BREAKING CHANGE

```
chore!: eliminar soporte para Node 6

BREAKING CHANGE: usar características de JavaScript no disponibles en Node 6.
```

### Commit sin cuerpo

```
docs: corregir ortografía de CHANGELOG
```

### Commit con ámbito

```
feat(lang): agregar idioma polaco
```

### Commit con cuerpo de múltiples párrafos y múltiples pies de página

```
fix: prevenir condición de carrera en las peticiones

Introducir un id de petición y una referencia a la última petición. Descartar
respuestas entrantes que no sean de la última petición.

Eliminar timeouts que se usaban para mitigar el problema de condición de carrera
pero que ahora son obsoletos.

Revisado-por: Z
Refs: #123
```

### Más Ejemplos en Español

```
feat(auth): agregar autenticación con OAuth2
```

```
fix(api): corregir validación de datos en el endpoint de usuarios
```

```
docs: actualizar documentación de instalación
```

```
style(ui): ajustar espaciado en el componente de navegación
```

```
refactor(database): optimizar consultas de búsqueda
```

```
perf(api): mejorar tiempo de respuesta de endpoints
```

```
test(auth): agregar pruebas unitarias para el módulo de login
```

```
chore(deps): actualizar dependencias a las últimas versiones
```

```
ci: agregar workflow de GitHub Actions para testing
```

```
build: configurar webpack para producción
```

## Guía de Tipos Comunes en Español

- **feat**: Nueva característica o funcionalidad
  - Ejemplo: `feat(usuario): agregar página de perfil`
  
- **fix**: Corrección de errores
  - Ejemplo: `fix(login): resolver problema de redirección después del login`
  
- **docs**: Cambios en documentación
  - Ejemplo: `docs: actualizar guía de contribución`
  
- **style**: Cambios de formato, espacios en blanco, etc. (no afectan el código)
  - Ejemplo: `style: formatear código según estándar del proyecto`
  
- **refactor**: Refactorización de código (sin cambiar funcionalidad)
  - Ejemplo: `refactor(api): simplificar lógica de validación`
  
- **perf**: Mejoras de rendimiento
  - Ejemplo: `perf(db): optimizar consultas de base de datos`
  
- **test**: Añadir o modificar tests
  - Ejemplo: `test(utils): agregar tests para funciones de validación`
  
- **build**: Cambios en el sistema de build o dependencias
  - Ejemplo: `build: actualizar webpack a v5`
  
- **ci**: Cambios en configuración de CI/CD
  - Ejemplo: `ci: agregar cache para node_modules en pipeline`
  
- **chore**: Tareas de mantenimiento
  - Ejemplo: `chore: limpiar archivos temporales`
  
- **revert**: Revertir un commit anterior
  - Ejemplo: `revert: revertir "feat: agregar nueva funcionalidad"`

## Especificación

Las palabras clave "DEBE", "NO DEBE", "REQUERIDO", "DEBERÍA", "NO DEBERÍA", "RECOMENDADO", "PUEDE", y "OPCIONAL" en este documento deben interpretarse como se describe en [RFC 2119](https://www.ietf.org/rfc/rfc2119.txt).

1.  Los commits DEBEN tener un prefijo con un tipo, que consiste en un sustantivo, `feat`, `fix`, etc., seguido del ámbito OPCIONAL, `!` OPCIONAL, y dos puntos y espacio REQUERIDOS.
2.  El tipo `feat` DEBE usarse cuando un commit añade una nueva característica a tu aplicación o librería.
3.  El tipo `fix` DEBE usarse cuando un commit representa una corrección de error para tu aplicación.
4.  Se PUEDE proporcionar un ámbito después de un tipo. Un ámbito DEBE consistir en un sustantivo que describe una sección del código entre paréntesis, ej., `fix(parser):`
5.  Una descripción DEBE seguir inmediatamente a los dos puntos y espacio después del prefijo tipo/ámbito. La descripción es un resumen corto de los cambios de código, ej., _fix: problema al parsear arrays cuando hay múltiples espacios en el string_.
6.  Se PUEDE proporcionar un cuerpo de commit más largo después de la descripción corta, proporcionando información contextual adicional sobre los cambios de código. El cuerpo DEBE comenzar una línea en blanco después de la descripción.
7.  Un cuerpo de commit es de forma libre y PUEDE consistir en cualquier número de párrafos separados por saltos de línea.
8.  Se PUEDEN proporcionar uno o más pies de página una línea en blanco después del cuerpo. Cada pie de página DEBE consistir en un token de palabra, seguido por un separador `:<espacio>` o `<espacio>#`, seguido por un valor de cadena.
9.  Un token de pie de página DEBE usar `-` en lugar de caracteres de espacio en blanco, ej., `Revisado-por`.
10. El valor de un pie de página PUEDE contener espacios y saltos de línea.
11. Los cambios importantes DEBEN indicarse en el prefijo tipo/ámbito de un commit, o como una entrada en el pie de página.
12. Si se incluye como pie de página, un cambio importante DEBE consistir en el texto en mayúsculas BREAKING CHANGE, seguido de dos puntos, espacio y descripción.
13. Si se incluye en el prefijo tipo/ámbito, los cambios importantes DEBEN indicarse con un `!` inmediatamente antes de `:`.
14. Se PUEDEN usar tipos distintos de `feat` y `fix` en tus mensajes de commit.
15. Las unidades de información que componen Conventional Commits NO DEBEN tratarse como sensibles a mayúsculas/minúsculas por los implementadores, con la excepción de BREAKING CHANGE que DEBE estar en mayúsculas.
16. BREAKING-CHANGE DEBE ser sinónimo de BREAKING CHANGE, cuando se usa como token en un pie de página.

## Por Qué Usar Conventional Commits

- Generar automáticamente CHANGELOGs.
- Determinar automáticamente un incremento de versión semántica (basado en los tipos de commits).
- Comunicar la naturaleza de los cambios a compañeros de equipo, el público y otras partes interesadas.
- Activar procesos de build y publicación.
- Facilitar que las personas contribuyan a tus proyectos, permitiéndoles explorar un historial de commits más estructurado.

## Notas Importantes

- Por defecto, se ejecutarán verificaciones pre-commit (definidas en `.pre-commit-config.yaml`) para asegurar la calidad del código
  - IMPORTANTE: NO OMITIR las verificaciones pre-commit
- SIEMPRE atribuir la autoría de código asistido por IA
- Si archivos específicos ya están en staging, el comando solo commitará esos archivos
- Si no hay archivos en staging, automáticamente añadirá todos los archivos modificados y nuevos
- El mensaje del commit se construirá basándose en los cambios detectados
- Antes de commitear, el comando revisará el diff para identificar si múltiples commits serían más apropiados
- Si se sugieren múltiples commits, ayudará a separar y commitear los cambios por separado
- Siempre revisa el diff del commit para asegurar que el mensaje coincida con los cambios
- **TODOS los mensajes de commit deben estar en ESPAÑOL** (descripción, cuerpo y pies de página)
- Los tipos de commit (feat, fix, etc.) se mantienen en inglés según el estándar

### Atribución de Autoría de Código Asistido por IA

Cuando uses herramientas de IA para generar código, puede ser beneficioso mantener transparencia sobre la autoría para fines de responsabilidad, revisión de código y auditoría. Esto se puede hacer fácilmente usando Git trailers que añaden metadatos estructurados al final de los mensajes de commit.

Esto se puede hacer añadiendo uno o más trailers personalizados en el mensaje de commit, como:

```
Asistente-modelo: Claude Code
```

Los trailers se pueden añadir manualmente al final del mensaje de commit, o usando el comando `git commit` con la opción `--trailer`:

```
git commit --message "feat: implementar nueva funcionalidad" --trailer "Asistente-modelo: Claude Code"
```

## Ejemplos Completos de Uso

### Ejemplo 1: Nueva Funcionalidad Simple
```
feat(usuarios): agregar página de perfil de usuario

Implementar nueva página que permite a los usuarios ver y editar
su información personal incluyendo nombre, email y foto de perfil.

Asistente-modelo: Claude Code
```

### Ejemplo 2: Corrección de Error
```
fix(api): corregir timeout en peticiones de búsqueda

Aumentar el tiempo de espera de 5s a 30s para consultas complejas
de búsqueda que requieren más tiempo de procesamiento.

Closes: #123
```

### Ejemplo 3: Cambio Importante
```
feat(auth)!: migrar a OAuth 2.0

BREAKING CHANGE: el sistema de autenticación anterior basado en tokens
personalizados ha sido reemplazado por OAuth 2.0. Los clientes existentes
necesitarán actualizar su implementación de autenticación.

Refs: #456
Revisado-por: María García
```

### Ejemplo 4: Refactorización
```
refactor(database): optimizar consultas de productos

Reescribir consultas SQL para usar índices apropiados y reducir
el número de joins, mejorando el tiempo de respuesta en un 60%.

Asistente-modelo: Claude Code
```

### Ejemplo 5: Documentación
```
docs: actualizar guía de instalación para desarrollo local

Agregar instrucciones detalladas para configurar el entorno de desarrollo
en macOS, Linux y Windows, incluyendo requisitos previos y solución de
problemas comunes.
```
