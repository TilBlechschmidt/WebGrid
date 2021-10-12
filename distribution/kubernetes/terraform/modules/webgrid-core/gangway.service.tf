resource "kubernetes_service" "gangway" {
  metadata {
    name      = var.name
    namespace = var.namespace
    labels    = local.labels.gangway
  }

  spec {
    port {
      name        = "http"
      target_port = "http"
      port        = var.service_port
    }

    selector = kubernetes_stateful_set.gangway.metadata[0].labels
  }
}
