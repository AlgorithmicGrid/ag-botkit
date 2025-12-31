# ag-botkit Deployment Runbook

## Table of Contents
1. [Pre-Deployment Checklist](#pre-deployment-checklist)
2. [Infrastructure Provisioning](#infrastructure-provisioning)
3. [Application Deployment](#application-deployment)
4. [Post-Deployment Validation](#post-deployment-validation)
5. [Rollback Procedures](#rollback-procedures)
6. [Troubleshooting](#troubleshooting)

---

## Pre-Deployment Checklist

### Prerequisites
- [ ] AWS CLI configured with appropriate credentials
- [ ] kubectl installed and configured
- [ ] Terraform >= 1.6 installed
- [ ] Docker installed for local testing
- [ ] Access to GitHub repository
- [ ] Access to container registry (ghcr.io)
- [ ] All secrets prepared (database passwords, API keys)

### Pre-Deployment Tasks
- [ ] Review change log and release notes
- [ ] Verify all CI/CD tests passed
- [ ] Backup current production state
- [ ] Notify stakeholders of deployment window
- [ ] Prepare rollback plan
- [ ] Schedule deployment during low-traffic period

---

## Infrastructure Provisioning

### Step 1: Initialize Terraform Backend

```bash
# Create S3 bucket for Terraform state
aws s3 mb s3://ag-botkit-terraform-state --region us-east-1

# Enable versioning
aws s3api put-bucket-versioning \
  --bucket ag-botkit-terraform-state \
  --versioning-configuration Status=Enabled

# Create DynamoDB table for state locking
aws dynamodb create-table \
  --table-name ag-botkit-terraform-locks \
  --attribute-definitions AttributeName=LockID,AttributeType=S \
  --key-schema AttributeName=LockID,KeyType=HASH \
  --provisioned-throughput ReadCapacityUnits=5,WriteCapacityUnits=5 \
  --region us-east-1
```

### Step 2: Deploy Infrastructure

```bash
cd /path/to/ag-botkit/infra/terraform

# Initialize Terraform
terraform init

# Create terraform.tfvars
cat > terraform.tfvars <<EOF
aws_region              = "us-east-1"
environment             = "production"
eks_node_desired_size   = 5
enable_rds_multi_az     = true
enable_eks_encryption   = true
EOF

# Plan deployment
terraform plan -out=tfplan

# Review plan carefully
less tfplan

# Apply infrastructure
terraform apply tfplan

# Save outputs
terraform output -json > ../outputs.json
```

**Expected Duration:** 30-45 minutes

### Step 3: Configure kubectl

```bash
# Get EKS cluster config
aws eks update-kubeconfig \
  --region us-east-1 \
  --name ag-botkit-production-eks

# Verify connectivity
kubectl cluster-info
kubectl get nodes
```

---

## Application Deployment

### Step 1: Create Namespace and Base Resources

```bash
cd /path/to/ag-botkit

# Apply namespace
kubectl apply -f deploy/k8s/namespace.yaml

# Verify namespace
kubectl get namespace ag-botkit
```

### Step 2: Configure Secrets

```bash
# Copy secrets template
cp deploy/k8s/secrets.yaml.template deploy/k8s/secrets.yaml

# Generate database password
DB_PASSWORD=$(openssl rand -base64 32)

# Edit secrets.yaml and replace placeholders
# IMPORTANT: Do not commit secrets.yaml

# Apply secrets
kubectl apply -f deploy/k8s/secrets.yaml

# Verify secrets (do not view values)
kubectl get secrets -n ag-botkit
```

### Step 3: Apply ConfigMaps

```bash
# Apply configuration
kubectl apply -f deploy/k8s/configmaps.yaml

# Verify ConfigMaps
kubectl get configmaps -n ag-botkit
kubectl describe configmap minibot-config -n ag-botkit
```

### Step 4: Deploy Database

```bash
# Deploy TimescaleDB StatefulSet
kubectl apply -f deploy/k8s/timescaledb-statefulset.yaml

# Wait for database to be ready (5-10 minutes)
kubectl rollout status statefulset/timescaledb -n ag-botkit --timeout=10m

# Verify database is running
kubectl get pods -n ag-botkit -l app=timescaledb
kubectl logs -n ag-botkit timescaledb-0 --tail=50

# Test database connectivity
kubectl run -it --rm psql-test \
  --image=postgres:15 \
  --restart=Never \
  -n ag-botkit \
  --env="PGPASSWORD=<password>" \
  -- psql -h timescaledb -U agbot -d ag_botkit -c "SELECT version();"
```

### Step 5: Deploy Applications

```bash
# Deploy monitor
kubectl apply -f deploy/k8s/monitor-deployment.yaml

# Wait for monitor rollout
kubectl rollout status deployment/monitor -n ag-botkit --timeout=5m

# Deploy minibot
kubectl apply -f deploy/k8s/minibot-deployment.yaml

# Wait for minibot rollout
kubectl rollout status deployment/minibot -n ag-botkit --timeout=5m
```

### Step 6: Configure Ingress

```bash
# Update ingress.yaml with your domain names
vim deploy/k8s/ingress.yaml

# Apply ingress
kubectl apply -f deploy/k8s/ingress.yaml

# Verify ingress
kubectl get ingress -n ag-botkit
kubectl describe ingress monitor-ingress -n ag-botkit
```

**Expected Duration:** 15-20 minutes

---

## Post-Deployment Validation

### Health Checks

```bash
# Check all pods
kubectl get pods -n ag-botkit -o wide

# Expected output:
# NAME                       READY   STATUS    RESTARTS   AGE
# monitor-xxxxx              1/1     Running   0          5m
# minibot-xxxxx              1/1     Running   0          5m
# timescaledb-0              1/1     Running   0          10m

# Check services
kubectl get svc -n ag-botkit

# Check logs
kubectl logs -n ag-botkit deployment/monitor --tail=100
kubectl logs -n ag-botkit deployment/minibot --tail=100
```

### Functional Tests

```bash
# Test monitor health endpoint
kubectl run curl-test --image=curlimages/curl:latest --rm -i --restart=Never -n ag-botkit -- \
  curl -v http://monitor:8080/health

# Expected: HTTP 200 OK

# Test database connectivity from pod
kubectl exec -it deployment/monitor -n ag-botkit -- \
  wget -O- http://timescaledb:5432 2>&1 | grep -i postgres

# Access dashboard (if ingress configured)
# Open browser to https://monitor.ag-botkit.example.com
```

### Metrics Validation

```bash
# Check Prometheus targets
kubectl port-forward -n ag-botkit svc/prometheus 9090:9090 &
# Open http://localhost:9090/targets

# Check Grafana dashboards
kubectl port-forward -n ag-botkit svc/grafana 3000:3000 &
# Open http://localhost:3000
# Login: admin / <GRAFANA_PASSWORD>
```

### Performance Baseline

- [ ] Monitor CPU usage: Should be < 50% under normal load
- [ ] Monitor memory usage: Should be < 70% under normal load
- [ ] RTDS lag: Should be < 100ms consistently
- [ ] Message rate: Should match expected throughput
- [ ] Database query latency: Should be < 50ms for typical queries

---

## Rollback Procedures

### Quick Rollback (Application Only)

```bash
# Rollback monitor deployment
kubectl rollout undo deployment/monitor -n ag-botkit

# Rollback minibot deployment
kubectl rollout undo deployment/minibot -n ag-botkit

# Verify rollback
kubectl rollout status deployment/monitor -n ag-botkit
kubectl rollout status deployment/minibot -n ag-botkit
```

### Full Rollback (Infrastructure + Application)

```bash
# 1. Take database snapshot first
kubectl exec -n ag-botkit timescaledb-0 -- \
  pg_dump -U agbot ag_botkit > backup-$(date +%Y%m%d-%H%M%S).sql

# 2. Destroy infrastructure
cd /path/to/ag-botkit/infra/terraform
terraform plan -destroy -out=destroy-plan
terraform apply destroy-plan

# 3. Restore previous infrastructure version
git checkout <previous-commit>
terraform init
terraform apply

# 4. Restore database
kubectl exec -i -n ag-botkit timescaledb-0 -- \
  psql -U agbot ag_botkit < backup-YYYYMMDD-HHMMSS.sql
```

### Emergency Stop

```bash
# Scale down all deployments
kubectl scale deployment --all --replicas=0 -n ag-botkit

# Trigger risk kill switch
kubectl exec -it deployment/minibot -n ag-botkit -- \
  curl -X POST http://localhost:9091/api/kill-switch/trigger
```

---

## Troubleshooting

### Pod Won't Start

```bash
# Describe pod for events
kubectl describe pod <pod-name> -n ag-botkit

# Common issues:
# - ImagePullBackOff: Check image name and registry credentials
# - CrashLoopBackOff: Check logs for application errors
# - Pending: Check resource quotas and node capacity
```

### Database Connection Issues

```bash
# Check database pod
kubectl get pod timescaledb-0 -n ag-botkit
kubectl logs timescaledb-0 -n ag-botkit

# Test connectivity from application pod
kubectl exec -it deployment/monitor -n ag-botkit -- /bin/sh
# Inside pod:
# nc -zv timescaledb 5432

# Check secrets
kubectl get secret database-credentials -n ag-botkit -o yaml
# Decode and verify connection string
```

### RTDS Connection Failures

```bash
# Check minibot logs
kubectl logs -n ag-botkit deployment/minibot --tail=200 | grep -i rtds

# Common issues:
# - Network policy blocking egress: Check network policies
# - DNS resolution: Test with nslookup from pod
# - Firewall: Verify security groups allow outbound HTTPS
```

### High Latency/Performance Issues

```bash
# Check resource usage
kubectl top pods -n ag-botkit
kubectl top nodes

# Check HPA status
kubectl get hpa -n ag-botkit

# Scale manually if needed
kubectl scale deployment/monitor --replicas=5 -n ag-botkit
```

### Disk Space Issues (Database)

```bash
# Check PVC status
kubectl get pvc -n ag-botkit

# Expand PVC (if supported by storage class)
kubectl patch pvc data-timescaledb-0 -n ag-botkit \
  -p '{"spec":{"resources":{"requests":{"storage":"200Gi"}}}}'

# Execute manual cleanup
kubectl exec -it timescaledb-0 -n ag-botkit -- psql -U agbot -d ag_botkit
# Inside psql:
# VACUUM FULL;
# SELECT drop_chunks('metrics', interval '90 days');
```

---

## Maintenance Tasks

### Regular Maintenance Schedule

**Daily:**
- Review Grafana dashboards for anomalies
- Check alert notifications
- Verify backup completion

**Weekly:**
- Review and optimize slow queries
- Check disk usage trends
- Update security patches (if available)

**Monthly:**
- Review and update capacity planning
- Test disaster recovery procedures
- Audit access logs
- Review and optimize Kubernetes resource requests/limits

### Database Maintenance

```bash
# Manual VACUUM
kubectl exec -it timescaledb-0 -n ag-botkit -- \
  psql -U agbot -d ag_botkit -c "VACUUM ANALYZE;"

# Check compression status
kubectl exec -it timescaledb-0 -n ag-botkit -- \
  psql -U agbot -d ag_botkit -c "SELECT * FROM timescaledb_information.compressed_chunk_stats;"

# Drop old chunks (adjust interval as needed)
kubectl exec -it timescaledb-0 -n ag-botkit -- \
  psql -U agbot -d ag_botkit -c "SELECT drop_chunks('metrics', interval '180 days');"
```

---

## Contact Information

**On-Call Engineer:** [PagerDuty/Phone]
**DevOps Team:** devops@ag-botkit.example.com
**Slack Channel:** #ag-botkit-ops

---

**Last Updated:** 2025-12-31
**Document Owner:** DevOps Team
**Review Frequency:** Monthly
