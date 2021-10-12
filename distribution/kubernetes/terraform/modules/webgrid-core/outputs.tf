output "service" {
  value       = kubernetes_service.gangway.metadata[0].name
  description = "Name of the service which represents the central entrypoint for the WebGrid"
}
