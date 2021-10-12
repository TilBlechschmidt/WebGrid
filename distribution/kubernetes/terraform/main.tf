provider "kubernetes" {
  config_path    = "~/.kube/config"
  config_context = "ppi.int"
}

# resource "kubernetes_namespace" "webgrid" {
#   metadata {
#     name = "webgrid-tf"
#   }
# }

locals {
  namespace = "selenium-test" # kubernetes_namespace.webgrid.metadata[0].name
}

module "redis" {
  source = "./modules/redis"

  name      = "webgrid-ki"
  namespace = local.namespace
}

module "mongo" {
  source = "./modules/mongo"

  name      = "webgrid-ki"
  namespace = local.namespace

  deploy_express = true

  node_selector = {
    "dedicated" = "selenium-ki"
  }

  tolerations = [
    {
      key      = "dedicated"
      operator = "Equal"
      value    = "selenium-ki"
      effect   = "NoSchedule"
    }
  ]
}

module "core" {
  source = "./modules/webgrid-core"

  name      = "webgrid-ki"
  namespace = local.namespace

  node_selector = {
    "dedicated" = "selenium-ki"
  }

  tolerations = [
    {
      key      = "dedicated"
      operator = "Equal"
      value    = "selenium-ki"
      effect   = "NoSchedule"
    }
  ]

  image = {
    repo        = "ghcr.io/tilblechschmidt/webgrid"
    tag         = "sha-0ef706f"
    pull_policy = "Always"
  }

  redis = "redis://${module.redis.service}"

  orchestrator = {
    permits = 1
  }
}

output "webgrid-endpoint" {
  value       = "${module.core.service}.${local.namespace}"
  description = "Service endpoint at which the grid is reachable"
}
