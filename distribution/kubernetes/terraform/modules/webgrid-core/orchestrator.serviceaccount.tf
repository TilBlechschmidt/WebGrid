resource "kubernetes_service_account" "orchestrator" {
  count = local.create_service_account ? 1 : 0

  metadata {
    name      = local.service_account_name
    namespace = var.namespace
    labels    = local.labels.orchestrator
  }
}

resource "kubernetes_role" "orchestrator" {
  count = local.create_service_account ? 1 : 0

  metadata {
    name      = local.service_account_name
    namespace = var.namespace
  }

  rule {
    api_groups = ["", "batch"]
    resources  = ["jobs", "pods"]
    verbs      = ["create", "list", "delete"]
  }
}

resource "kubernetes_role_binding" "orchestrator" {
  count = local.create_service_account ? 1 : 0

  metadata {
    name      = local.service_account_name
    namespace = var.namespace
  }

  subject {
    kind      = "ServiceAccount"
    name      = kubernetes_service_account.orchestrator[0].metadata[0].name
    namespace = var.namespace
  }

  role_ref {
    kind      = "Role"
    name      = kubernetes_role.orchestrator[0].metadata[0].name
    api_group = "rbac.authorization.k8s.io"
  }
}
