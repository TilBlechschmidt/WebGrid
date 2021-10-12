terraform {
  required_version = ">= 0.15.0"

  required_providers {
    kubernetes = {
      source  = "hashicorp/kubernetes"
      version = ">= 2.5.0"
    }
  }

  experiments = [module_variable_optional_attrs]
}
