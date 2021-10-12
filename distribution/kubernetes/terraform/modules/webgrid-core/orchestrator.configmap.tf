resource "kubernetes_config_map" "orchestrator" {
  metadata {
    name      = "${var.name}-orchestrator"
    namespace = var.namespace
    labels    = local.labels.orchestrator
  }

  data = {
    "job.yaml" = <<-EOT
    apiVersion: batch/v1
    kind: Job
    metadata:
      name: {{job_name}}
      labels:
        instance: ${var.name}
        dev.webgrid/component: node
        dev.webgrid/session.id: {{session_id}}
        dev.webgrid/provisioner.instance: {{provisioner_instance}}
    spec:
      backoffLimit: 0
      template:
        metadata:
          labels:
            instance: ${var.name}
            dev.webgrid/component: node
            dev.webgrid/session.id: "{{session_id}}"
        spec:
          restartPolicy: Never
          volumes:
            - name: dshm
              emptyDir:
                medium: Memory
          tolerations:
            %{for toleration in var.tolerations}
            - key: ${toleration.key}
              operator: ${toleration.operator}
              value: ${toleration.value}
              effect: ${toleration.effect}
            %{endfor}
          nodeSelector:
            %{for k, v in var.node_selector}
            ${k}: ${v}
            %{endfor}
          containers:
            - name: {{job_name}}
              image: {{image_name}}
              imagePullPolicy: ${var.image.pull_policy}
              ports:
                - name: http
                  containerPort: 48049
                  protocol: TCP
              env:
                - name: RUST_LOG
                  value: ${var.log}
                - name: ID
                  value: "{{session_id}}"
                - name: CAPABILITIES
                  value: '{{capabilities}}'
                - name: REDIS
                  value: "${var.redis}"
                - name: HOST
                  valueFrom:
                    fieldRef:
                      fieldPath: status.podIP
                - name: BIND_TIMEOUT
                  value: 600
                %{if var.session.startup_timeout != null}
                - name: STARTUP_TIMEOUT
                  value: "${var.session.startup_timeout}"
                %{endif}
                %{if var.session.initial_timeout != null}
                - name: INITIAL_TIMEOUT
                  value: "${var.session.initial_timeout}"
                %{endif}
                %{if var.session.idle_timeout != null}
                - name: IDLE_TIMEOUT
                  value: "${var.session.idle_timeout}"
                %{endif}
                %{if var.session.resolution != null}
                - name: RESOLUTION
                  value: "${var.session.resolution}"
                %{endif}
                %{if var.session.crf != null}
                - name: CRF
                  value: "${var.session.crf}"
                %{endif}
                %{if var.session.max_bitrate != null}
                - name: MAX_BITRATE
                  value: "${var.session.max_bitrate}"
                %{endif}
                %{if var.session.framerate != null}
                - name: FRAMERATE
                  value: "${var.session.framerate}"
                %{endif}
                %{if var.session.segment_duration != null}
                - name: SEGMENT_DURATION
                  value: "${var.session.segment_duration}"
                %{endif}
                %{if var.storage != null}
                - name: STORAGE
                  value: ${var.storage}
                %{endif}
              volumeMounts:
                - mountPath: /dev/shm
                  name: dshm
    EOT
  }
}
