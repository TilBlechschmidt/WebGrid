resource "kubernetes_stateful_set" "gangway" {
  metadata {
    name      = "${var.name}-gangway"
    namespace = var.namespace
    labels    = local.labels.gangway
  }

  spec {
    service_name          = "${var.name}-gangway"
    pod_management_policy = "Parallel"
    replicas              = var.replicas.gangway

    selector {
      match_labels = local.labels.gangway
    }

    template {
      metadata {
        labels = local.labels.gangway
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

          name = "gangway"
          args = ["gangway", "--status-server", "47002"]

          port {
            name           = "http"
            container_port = 48048
          }

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
            for_each = var.storage[*]

            content {
              name  = "STORAGE"
              value = var.storage
            }
          }

          dynamic "env" {
            for_each = var.gangway.cache_size[*]

            content {
              name  = "CACHE_SIZE"
              value = var.gangway.cache_size
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
