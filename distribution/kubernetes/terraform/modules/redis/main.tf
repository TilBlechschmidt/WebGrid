variable "name" {
  type    = string
  default = "webgrid"
}

variable "namespace" {
  type = string
}

locals {
  labels = {
    instance = var.name
  }
}

resource "kubernetes_config_map" "redis" {
  metadata {
    name      = "${var.name}-redis"
    namespace = var.namespace
    labels    = local.labels
  }

  data = {
    "redis.conf" = <<-EOT
    maxmemory 100mb

    # RDB Snapshots
    save 900 1
    save 300 10
    save 60 10000
    
    dbfilename dump.rdb
    dir /data

    # AOF Persistence
    appendonly yes
    appendfilename "appendonly.aof"
    appendfsync everysec
    auto-aof-rewrite-percentage 100
    auto-aof-rewrite-min-size 64mb
    aof-load-truncated yes
    aof-use-rdb-preamble yes
    EOT
  }
}

resource "kubernetes_stateful_set" "redis" {
  metadata {
    name      = "${var.name}-redis"
    namespace = var.namespace
    labels    = local.labels
  }

  spec {
    service_name          = "${var.name}-redis"
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
        volume {
          name = "redis-config"
          config_map {
            name = kubernetes_config_map.redis.metadata[0].name
          }
        }

        // TODO Add persistent volume support

        volume {
          name = "redis-persistence"
          // TODO Add persistent volume support
          empty_dir {
            medium = "Memory"
          }
        }

        container {
          image = "redis:6.2-alpine"

          name = "redis"
          args = ["/config/redis.conf"]

          port {
            name           = "redis"
            container_port = 6379
          }

          volume_mount {
            name       = "redis-config"
            mount_path = "/config"
          }

          volume_mount {
            name       = "redis-persistence"
            mount_path = "/data"
          }
        }
      }
    }
  }
}

resource "kubernetes_service" "redis" {
  metadata {
    name      = "${var.name}-redis"
    namespace = var.namespace
    labels    = local.labels
  }

  spec {
    port {
      port        = 6379
      name        = "redis"
      target_port = "redis"
    }

    selector = kubernetes_stateful_set.redis.metadata[0].labels
  }
}

output "service" {
  value       = kubernetes_service.redis.metadata[0].name
  description = "Name of the service at which the redis is reachable on the default port"
}
