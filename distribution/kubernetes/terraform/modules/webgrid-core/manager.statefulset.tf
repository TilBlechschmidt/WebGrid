resource "kubernetes_stateful_set" "manager" {
  metadata {
    name      = "${var.name}-manager"
    namespace = var.namespace
    labels    = local.labels.manager
  }

  spec {
    service_name          = "${var.name}-manager"
    pod_management_policy = "Parallel"
    replicas              = var.replicas.manager

    selector {
      match_labels = local.labels.manager
    }

    template {
      metadata {
        labels = local.labels.manager
      }

      spec {
        automount_service_account_token = false

        node_selector = var.node_selector

        dynamic "toleration" {
          for_each = var.tolerations
          content {
            key      = toleration.value["key"]
            operator = toleration.value["operator"]
            value    = toleration.value["value"]
            effect   = toleration.value["effect"]
          }
        }

        container {
          image             = local.image
          image_pull_policy = var.image.pull_policy

          name = "manager"
          args = ["manager", "--status-server", "47002"]

          port {
            name           = "status"
            container_port = 47002
          }

          env {
            name = "ID"
            value_from {
              field_ref {
                field_path = "metadata.name"
              }
            }
          }

          env {
            name  = "RUST_LOG"
            value = var.log
          }

          env {
            name  = "REDIS"
            value = var.redis
          }

          dynamic "env" {
            for_each = var.manager.required_metadata[*]

            content {
              name  = "REQUIRED_METADATA"
              value = var.manager.required_metadata
            }
          }

          liveness_probe {
            tcp_socket {
              port = "status"
            }
          }

          readiness_probe {
            http_get {
              port = "status"
              path = "/status"
            }
          }
        }
      }
    }
  }
}
