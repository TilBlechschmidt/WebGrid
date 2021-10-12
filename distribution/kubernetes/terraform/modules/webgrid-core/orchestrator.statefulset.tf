resource "kubernetes_stateful_set" "orchestrator" {
  metadata {
    name      = "${var.name}-orchestrator"
    namespace = var.namespace
    labels    = local.labels.orchestrator
  }

  spec {
    service_name          = "${var.name}-orchestrator"
    pod_management_policy = "Parallel"
    replicas              = var.replicas.orchestrator

    selector {
      match_labels = local.labels.orchestrator
    }

    template {
      metadata {
        labels = local.labels.orchestrator
      }

      spec {
        service_account_name = kubernetes_service_account.orchestrator[0].metadata[0].name

        // TODO Add affinity, tolerations

        volume {
          name = "orchestrator-config"
          config_map {
            name = kubernetes_config_map.orchestrator.metadata[0].name
          }
        }

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

          name = "orchestrator"
          args = ["orchestrator", "kubernetes", "--status-server", "47002"]

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

          env {
            name  = "PERMITS"
            value = var.orchestrator.permits
          }

          env {
            name  = "IMAGES"
            value = local.session_images
          }

          env {
            name  = "WEBGRID_CONFIG_DIR"
            value = "/configs"
          }

          env {
            name  = "WEBGRID_RESOURCE_PREFIX"
            value = "${var.name}-"
          }

          env {
            name  = "NAMESPACE"
            value = var.namespace
          }

          env {
            name  = "ORCHESTRATOR_CONFIG_CHANGE"
            value = sha1(jsonencode(merge(kubernetes_config_map.orchestrator.data)))
          }

          volume_mount {
            name       = "orchestrator-config"
            mount_path = "/configs"
            read_only  = true
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
