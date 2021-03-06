{{/* vim: set filetype=mustache: */}}
{{/*
Expand the name of the chart.
*/}}
{{- define "web-grid.name" -}}
{{- default .Chart.Name .Values.nameOverride | trunc 63 | trimSuffix "-" }}
{{- end }}

{{/*
Create a default fully qualified app name.
We truncate at 63 chars because some Kubernetes name fields are limited to this (by the DNS naming spec).
If release name contains chart name it will be used as a full name.
*/}}
{{- define "web-grid.fullname" -}}
{{- if .Values.fullnameOverride }}
{{- .Values.fullnameOverride | trunc 63 | trimSuffix "-" }}
{{- else }}
{{- $name := default .Chart.Name .Values.nameOverride }}
{{- if contains $name .Release.Name }}
{{- .Release.Name | trunc 63 | trimSuffix "-" }}
{{- else }}
{{- printf "%s-%s" .Release.Name $name | trunc 63 | trimSuffix "-" }}
{{- end }}
{{- end }}
{{- end }}

{{/*
Create chart name and version as used by the chart label.
*/}}
{{- define "web-grid.chart" -}}
{{- printf "%s-%s" .Chart.Name .Chart.Version | replace "+" "_" | trunc 63 | trimSuffix "-" }}
{{- end }}

{{/*
Common labels
*/}}
{{- define "web-grid.labels" -}}
helm.sh/chart: {{ include "web-grid.chart" . }}
{{ include "web-grid.selectorLabels" . }}
{{- if .Chart.AppVersion }}
app.kubernetes.io/version: {{ .Chart.AppVersion | quote }}
{{- end }}
app.kubernetes.io/managed-by: {{ .Release.Service }}
{{- end }}

{{/*
Selector labels
*/}}
{{- define "web-grid.selectorLabels" -}}
app.kubernetes.io/name: {{ include "web-grid.name" . }}
app.kubernetes.io/instance: {{ .Release.Name }}
{{- end }}

{{/*
Create a name for the redis service
*/}}
{{- define "web-grid.redisServiceName" -}}
{{- printf "%s-redis" (include "web-grid.fullname" .) }}
{{- end }}
{{- define "web-grid.redisURL" -}}
{{- printf "redis://%s/" (include "web-grid.redisServiceName" .) }}
{{- end }}

{{/*
Create the name of the service account to use
*/}}
{{- define "web-grid.serviceAccountName" -}}
{{- if .Values.serviceAccount.create }}
{{- default (include "web-grid.fullname" .) .Values.serviceAccount.name }}
{{- else }}
{{- default "default" .Values.serviceAccount.name }}
{{- end }}
{{- end }}

{{/*
Create the name of the persistent volume claim to use
*/}}
{{- define "web-grid.recordingPVCName" -}}
{{- if .Values.recording.persistentVolumeClaim.create }}
{{- default (include "web-grid.fullname" .) .Values.recording.persistentVolumeClaim.name }}
{{- else }}
{{- default "default" .Values.recording.persistentVolumeClaim.name }}
{{- end }}
{{- end }}

{{/*
Allow customization of the image tag used
*/}}
{{- define "web-grid.imageTag" -}}
{{- default .Chart.AppVersion .Values.image.tag }}
{{- end }}

{{/*
Create the name of the telemetry endpoint
*/}}
{{- define "web-grid.telemetryEndpoint" -}}
{{- if .Values.telemetry.demo }}
{{- printf "https://%s-telemetry:4317" (include "web-grid.fullname" .) }}
{{- else }}
{{ .Values.telemetry.endpoint }}
{{- end }}
{{- end }}
