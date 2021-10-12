// General configuration

variable "name" {
  type    = string
  default = "webgrid"
}

variable "namespace" {
  type = string
}

variable "service_port" {
  type    = number
  default = 80
}

variable "replicas" {
  type = object({
    gangway      = number
    manager      = number
    orchestrator = number
  })
  default = {
    gangway      = 1
    manager      = 1
    orchestrator = 1
  }
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

// Image configuration

variable "image" {
  type = object({
    repo        = string,
    tag         = string,
    pull_policy = string,
  })
  default = {
    repo        = "webgrid"
    tag         = "v0.5.1-beta"
    pull_policy = "IfNotPresent"
  }
}

// External services

variable "redis" {
  type = string
}

variable "storage" {
  type    = string
  default = null
}

// WebGrid configuration values

variable "log" {
  type    = string
  default = "info,hyper=warn,warp=warn,sqlx=warn,tower=warn,h2=warn"
}

variable "gangway" {
  type = object({
    cache_size = optional(number)
  })
  default = {}
}

variable "manager" {
  type = object({
    required_metadata = optional(string)
  })
  default = {}
}

variable "orchestrator" {
  type = object({
    permits              = number
    service_account_name = optional(string)
  })
}

variable "session" {
  type = object({
    startup_timeout  = optional(number)
    initial_timeout  = optional(number)
    idle_timeout     = optional(number)
    resolution       = optional(string)
    crf              = optional(string)
    max_bitrate      = optional(string)
    framerate        = optional(string)
    segment_duration = optional(string)
  })
  default = {}
}

// Derived values

locals {
  image                  = "${var.image.repo}/core:${var.image.tag}"
  session_images         = "${var.image.repo}/node-firefox:${var.image.tag}=firefox::68.7.0esr,${var.image.repo}/node-chrome:${var.image.tag}=chrome::81.0.4044.122"
  create_service_account = var.orchestrator.service_account_name == null
  service_account_name   = local.create_service_account ? "${var.name}-orchestrator" : var.orchestrator.service_account_name
  labels = {
    gangway = {
      instance                = var.name
      "dev.webgrid/component" = "gangway"
    }
    manager = {
      instance                = var.name
      "dev.webgrid/component" = "manager"
    }
    orchestrator = {
      instance                = var.name
      "dev.webgrid/component" = "orchestrator"
    }
  }
}
