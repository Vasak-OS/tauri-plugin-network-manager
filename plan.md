# Plan de Faltantes - tauri-plugin-network-manager

Objetivo: consolidar el plugin para produccion despues de completar el roadmap funcional Wi-Fi + VPN.

## Estado actual
- [x] Roadmap funcional Fase 0-6 completado (backend, comandos, permisos, frontend, tests de humo, documentacion base).
- [ ] Hardening de seguridad completado
- [ ] Resiliencia operativa completada
- [ ] Observabilidad de produccion completada
- [ ] Tests de integracion E2E completados
- [ ] Contrato API estable y versionado formal completado
- [ ] Release engineering automatizado completado

## Fase 7 - Seguridad y hardening
- [ ] Validar configuraciones VPN por tipo antes de ejecutar comandos:
	- [ ] WireGuard: campos minimos requeridos
	- [ ] OpenVPN: gateway/certificados/credenciales requeridas segun modo
	- [ ] Generic: validacion de llaves permitidas
- [ ] Sanitizar errores para no exponer secretos:
	- [ ] Remover passwords/tokens/private keys de mensajes de error
	- [ ] Remover secretos de eventos emitidos
	- [ ] Remover secretos de logs de debug
- [ ] Politica de manejo de secretos en memoria:
	- [ ] Evitar persistir secretos en estructuras de larga vida
	- [ ] Limpiar buffers temporales cuando aplique
- [ ] Revisar permisos por principio de minimo privilegio:
	- [ ] Confirmar set default minimo
	- [ ] Documentar permisos recomendados por entorno (dev/prod)

Entregables:
- [ ] Guía de seguridad aplicada (README + notas de implementacion)
- [ ] Checklist de no-exposicion de secretos verificado

## Fase 8 - Contrato API, compatibilidad y versionado
- [ ] Congelar contrato de API publica del SDK:
	- [ ] Payloads de funciones Wi-Fi
	- [ ] Payloads de funciones VPN
	- [ ] Payloads de eventos VPN
	- [ ] Catalogo de codigos de error
- [ ] Agregar tests de contrato para prevenir breaking changes:
	- [ ] Formas de respuesta esperadas
	- [ ] Campos opcionales/obligatorios
	- [ ] Errores tipados esperados
- [ ] Definir politica de semver del plugin:
	- [ ] Criterios de major/minor/patch
	- [ ] Regla de deprecaciones

Entregables:
- [ ] Documento de contrato API versionado
- [ ] Suite de tests de contrato en CI

## Fase 9 - Resiliencia operativa
- [ ] Manejo de reinicio de NetworkManager y reconexion D-Bus:
	- [ ] Reintento de suscripcion de eventos
	- [ ] Rehidratacion de estado interno
- [ ] Idempotencia de comandos mutantes:
	- [ ] connect/disconnect Wi-Fi idempotentes
	- [ ] connect/disconnect VPN idempotentes
- [ ] Mejorar semantica de estados transitorios:
	- [ ] Estados connecting/disconnecting consistentes
	- [ ] Tiempo maximo y fallback a failed/unknown
- [ ] Estrategia de retry con backoff para operaciones recuperables

Entregables:
- [ ] Plugin estable ante reinicios de servicio de red
- [ ] Menor tasa de errores intermitentes en operaciones mutantes

## Fase 10 - Observabilidad y diagnostico
- [ ] Instrumentar metricas de uso y salud:
	- [ ] Exito/fallo por comando
	- [ ] Latencia por comando
	- [ ] Eventos de reconexion
- [ ] Mejorar trazabilidad de errores:
	- [ ] Correlation id por operacion
	- [ ] Contexto minimo util sin datos sensibles
- [ ] Modo de diagnostico controlado por configuracion
- [ ] Estandarizar razones de fallos para UI (mapping estable)

Entregables:
- [ ] Base de telemetria local lista para soporte tecnico
- [ ] Matriz de errores y acciones recomendadas para UI

## Fase 11 - Pruebas de integracion E2E
- [ ] Crear entorno de pruebas Linux reproducible:
	- [ ] Perfil para Ubuntu
	- [ ] Perfil para Arch
- [ ] Agregar pruebas E2E de flujos reales:
	- [ ] Ciclo completo Wi-Fi (scan/connect/disconnect)
	- [ ] Ciclo completo VPN (create/connect/status/disconnect/delete)
- [ ] Agregar pruebas de errores reales:
	- [ ] Credenciales invalidas
	- [ ] Plugin VPN no disponible
	- [ ] Perfil inexistente/corrupto
- [ ] Integrar ejecucion en CI

Entregables:
- [ ] Suite E2E estable en CI Linux
- [ ] Evidencia de no-regresion en flujos criticos

## Fase 12 - DX del SDK y ergonomia frontend
- [ ] Helpers tipados para suscripcion de eventos:
	- [ ] onVpnChanged
	- [ ] onVpnConnected
	- [ ] onVpnDisconnected
	- [ ] onVpnFailed
- [ ] Utilidades de normalizacion de estado para UI:
	- [ ] Helpers para badges/estado visual
	- [ ] Helpers para retry sugerido por error
- [ ] Ejemplos de integracion listos para copiar

Entregables:
- [ ] SDK mas ergonomico para apps Tauri
- [ ] Menor codigo boilerplate en frontend consumidor

## Fase 13 - Release engineering
- [ ] Pipeline de release automatizado:
	- [ ] Checks obligatorios (build/test/lint)
	- [ ] Publicacion npm controlada por tag
	- [ ] Release notes automaticas
- [ ] Verificacion de artefactos:
	- [ ] Integridad de bundles
	- [ ] Tipos .d.ts correctos en paquete publicado
- [ ] Definir flujo de soporte post-release:
	- [ ] Plantilla de bug report
	- [ ] SLA de triage inicial

Entregables:
- [ ] Proceso de release repetible y auditable
- [ ] Menor riesgo operativo en publicaciones

## Secuencia recomendada
- [ ] Sprint A: Fase 7 + Fase 8
- [ ] Sprint B: Fase 9 + Fase 10
- [ ] Sprint C: Fase 11
- [ ] Sprint D: Fase 12 + Fase 13

## Registro de avance
- [ ] Fase 7 completada
- [ ] Fase 8 completada
- [ ] Fase 9 completada
- [ ] Fase 10 completada
- [ ] Fase 11 completada
- [ ] Fase 12 completada
- [ ] Fase 13 completada
