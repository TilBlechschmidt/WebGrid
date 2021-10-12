variable "name" {
  type    = string
  default = "webgrid"
}

variable "namespace" {
  type = string
}

variable "deploy_express" {
  type    = bool
  default = false
}

variable "node_selector" {
  type    = map(string)
  default = {}
}

variable "tolerations" {
  type = list(object({
    key      = string
    operator = string
    value    = string
    effect   = string
  }))
  default = []
}

locals {
  labels = {
    instance = var.name
  }
}

resource "kubernetes_stateful_set" "mongo" {
  metadata {
    name      = "${var.name}-mongo"
    namespace = var.namespace
    labels    = local.labels
  }

  spec {
    service_name          = "${var.name}-mongo"
    pod_management_policy = "Parallel"
    replicas              = 1

    selector {
      match_labels = local.labels
    }

    template {
      metadata {
        labels = local.labels
      }

      spec {
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

        volume {
          name = "mongo-persistence"
          // TODO Add persistent volume support
          empty_dir {
            medium = "Memory"
          }
        }

        container {
          image = "mongo:5.0.3"
          name  = "mongo"

          port {
            name           = "mongo"
            container_port = 27017
          }

          volume_mount {
            name       = "mongo-persistence"
            mount_path = "/data"
          }
        }

        dynamic "container" {
          for_each = var.deploy_express ? [1] : []

          content {
            image = "mongo-express"
            name  = "express"

            port {
              name           = "express"
              container_port = 8081
            }

            env {
              name  = "ME_CONFIG_MONGODB_URL"
              value = "mongodb://localhost:27017"
            }
          }
        }

      }
    }
  }
}

resource "kubernetes_service" "mongo" {
  metadata {
    name      = "${var.name}-mongo"
    namespace = var.namespace
    labels    = local.labels
  }

  spec {
    port {
      port        = 27017
      name        = "mongo"
      target_port = "mongo"
    }

    dynamic "port" {
      for_each = var.deploy_express ? [1] : []

      content {
        port        = 8081
        name        = "express"
        target_port = "express"
      }
    }

    selector = kubernetes_stateful_set.mongo.metadata[0].labels
  }
}

output "service" {
  value       = kubernetes_service.mongo.metadata[0].name
  description = "Name of the service at which the mongoDB is reachable on the default port"
}
