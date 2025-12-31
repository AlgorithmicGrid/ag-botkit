# ag-botkit Deployment Guide

This directory contains all production deployment configurations for ag-botkit, including Docker containers, Kubernetes manifests, CI/CD pipelines, and infrastructure as code.

## Table of Contents

- [Quick Start](#quick-start)
- [Directory Structure](#directory-structure)
- [Local Development](#local-development)
- [Production Deployment](#production-deployment)
- [Monitoring & Observability](#monitoring--observability)
- [Security](#security)
- [Troubleshooting](#troubleshooting)

---

## Quick Start

### Local Development with Docker Compose

```bash
# 1. Clone repository
git clone https://github.com/your-org/ag-botkit.git
cd ag-botkit

# 2. Set up environment variables
cp deploy/docker/.env.example deploy/docker/.env
# Edit .env with your configuration

# 3. Start services
cd deploy/docker
docker-compose up -d

# 4. Access services
# Monitor Dashboard: http://localhost:8080
# Grafana: http://localhost:3000 (admin/admin)
# Prometheus: http://localhost:9090
# TimescaleDB: localhost:5432

# 5. View logs
docker-compose logs -f

# 6. Stop services
docker-compose down
```

### Production Deployment

For production deployment, see [Production Deployment](#production-deployment) section below.

---

## Directory Structure

```
deploy/
├── docker/                     # Docker configurations
│   ├── Dockerfile.monitor     # Monitor dashboard image
│   ├── Dockerfile.minibot     # Minibot demo image
│   ├── Dockerfile.risk        # Risk library image
│   ├── docker-compose.yml     # Local development stack
│   └── .dockerignore          # Docker ignore rules
│
└── k8s/                       # Kubernetes manifests
    ├── namespace.yaml         # Namespace and RBAC
    ├── configmaps.yaml        # Application configuration
    ├── secrets.yaml.template  # Secrets template
    ├── monitor-deployment.yaml      # Monitor service
    ├── minibot-deployment.yaml      # Minibot service
    ├── timescaledb-statefulset.yaml # Database
    └── ingress.yaml          # External access

infra/
├── terraform/                 # Infrastructure as Code
│   ├── main.tf               # Main configuration
│   ├── variables.tf          # Input variables
│   ├── outputs.tf            # Output values
│   ├── vpc.tf                # VPC and networking
│   ├── eks.tf                # EKS cluster
│   └── rds.tf                # RDS TimescaleDB
│
├── monitoring/               # Monitoring stack
│   ├── prometheus/
│   │   └── config.yaml       # Prometheus configuration
│   ├── grafana/
│   │   └── dashboards/       # Grafana dashboards
│   └── alerts/
│       └── rules.yaml        # Alert rules
│
└── ops/                      # Operational procedures
    └── runbooks/
        ├── deployment-runbook.md    # Deployment procedures
        └── disaster-recovery.md     # DR procedures
```

---

## Local Development

### Prerequisites

- Docker >= 20.10
- Docker Compose >= 2.0
- 8GB RAM minimum
- 20GB disk space

### Building Images Locally

```bash
# Build all images
docker-compose -f deploy/docker/docker-compose.yml build

# Build specific service
docker-compose -f deploy/docker/docker-compose.yml build monitor

# Build without cache
docker-compose -f deploy/docker/docker-compose.yml build --no-cache
```

### Development Workflow

```bash
# Start with logs
docker-compose -f deploy/docker/docker-compose.yml up

# Start in background
docker-compose -f deploy/docker/docker-compose.yml up -d

# View logs
docker-compose -f deploy/docker/docker-compose.yml logs -f minibot

# Restart specific service
docker-compose -f deploy/docker/docker-compose.yml restart monitor

# Execute command in container
docker-compose -f deploy/docker/docker-compose.yml exec monitor sh

# Stop all services
docker-compose -f deploy/docker/docker-compose.yml down

# Stop and remove volumes (WARNING: deletes data)
docker-compose -f deploy/docker/docker-compose.yml down -v
```

### Database Access

```bash
# Connect to TimescaleDB
docker-compose -f deploy/docker/docker-compose.yml exec timescaledb \
  psql -U agbot -d ag_botkit

# Run SQL file
docker-compose -f deploy/docker/docker-compose.yml exec -T timescaledb \
  psql -U agbot -d ag_botkit < schema.sql

# Backup database
docker-compose -f deploy/docker/docker-compose.yml exec timescaledb \
  pg_dump -U agbot ag_botkit > backup.sql

# Restore database
cat backup.sql | docker-compose -f deploy/docker/docker-compose.yml exec -T timescaledb \
  psql -U agbot -d ag_botkit
```

---

## Production Deployment

### Prerequisites

- AWS Account with appropriate permissions
- Terraform >= 1.6
- kubectl >= 1.28
- AWS CLI configured
- Domain name (for ingress)
- SSL certificates (or cert-manager)

### Step 1: Infrastructure Provisioning

```bash
cd infra/terraform

# Initialize Terraform
terraform init

# Create production workspace
terraform workspace new production
terraform workspace select production

# Configure variables
cat > terraform.tfvars <<EOF
aws_region              = "us-east-1"
environment             = "production"
eks_node_desired_size   = 5
enable_rds_multi_az     = true
enable_eks_encryption   = true
allowed_cidr_blocks     = ["<your-office-ip>/32"]
EOF

# Plan deployment
terraform plan -out=tfplan

# Review plan carefully
terraform show tfplan

# Apply infrastructure (this takes 30-45 minutes)
terraform apply tfplan

# Save outputs
terraform output -json > outputs.json
```

### Step 2: Configure Kubernetes Access

```bash
# Get cluster credentials
aws eks update-kubeconfig \
  --region us-east-1 \
  --name ag-botkit-production-eks

# Verify access
kubectl get nodes
kubectl cluster-info
```

### Step 3: Configure Secrets

```bash
cd ../../deploy/k8s

# Copy secrets template
cp secrets.yaml.template secrets.yaml

# Generate strong passwords
openssl rand -base64 32

# Edit secrets.yaml and replace ALL placeholders
# IMPORTANT: Never commit secrets.yaml

# Apply secrets
kubectl apply -f secrets.yaml

# Verify (without displaying values)
kubectl get secrets -n ag-botkit
```

### Step 4: Deploy Applications

```bash
# Apply in order (dependencies matter)

# 1. Namespace and base resources
kubectl apply -f namespace.yaml

# 2. ConfigMaps
kubectl apply -f configmaps.yaml

# 3. Database (this takes 5-10 minutes)
kubectl apply -f timescaledb-statefulset.yaml
kubectl rollout status statefulset/timescaledb -n ag-botkit --timeout=10m

# 4. Monitor service
kubectl apply -f monitor-deployment.yaml
kubectl rollout status deployment/monitor -n ag-botkit --timeout=5m

# 5. Minibot service
kubectl apply -f minibot-deployment.yaml
kubectl rollout status deployment/minibot -n ag-botkit --timeout=5m

# 6. Ingress (update domains first!)
kubectl apply -f ingress.yaml
```

### Step 5: Verify Deployment

```bash
# Check all pods
kubectl get pods -n ag-botkit

# Expected output:
# NAME                       READY   STATUS    RESTARTS   AGE
# monitor-xxxxx              1/1     Running   0          5m
# minibot-xxxxx              1/1     Running   0          5m
# timescaledb-0              1/1     Running   0          10m

# Check services
kubectl get svc -n ag-botkit

# Check ingress
kubectl get ingress -n ag-botkit

# Test health endpoints
kubectl run curl-test --image=curlimages/curl:latest --rm -i --restart=Never -n ag-botkit -- \
  curl http://monitor:8080/health
```

---

## CI/CD Pipeline

### GitHub Actions Workflows

The repository includes two main workflows:

**1. CI Workflow (`.github/workflows/ci.yml`)**
- Triggered on: Push to main/develop, Pull requests
- Steps:
  - Lint (Rust, Go, C)
  - Test (all components)
  - Build verification
  - Security audit

**2. Deploy Workflow (`.github/workflows/deploy.yml`)**
- Triggered on: Push to main, Tags, Manual dispatch
- Steps:
  - Build Docker images
  - Push to container registry (ghcr.io)
  - Security scan (Trivy)
  - Deploy to staging/production
  - Smoke tests

### Setting Up CI/CD

```bash
# 1. Configure GitHub secrets (Settings > Secrets)
KUBE_CONFIG_STAGING     # Base64-encoded kubeconfig for staging
KUBE_CONFIG_PRODUCTION  # Base64-encoded kubeconfig for production
DB_PASSWORD             # Database password
GRAFANA_PASSWORD        # Grafana admin password
SLACK_WEBHOOK_URL       # For notifications (optional)

# 2. Generate kubeconfig
kubectl config view --raw --minify | base64

# 3. Test workflow locally (using act)
act -j build-and-push

# 4. Create release
git tag v1.0.0
git push origin v1.0.0
```

### Manual Deployment

```bash
# Build images
docker build -f deploy/docker/Dockerfile.monitor -t ghcr.io/your-org/ag-botkit-monitor:v1.0.0 .
docker build -f deploy/docker/Dockerfile.minibot -t ghcr.io/your-org/ag-botkit-minibot:v1.0.0 .

# Push to registry
docker push ghcr.io/your-org/ag-botkit-monitor:v1.0.0
docker push ghcr.io/your-org/ag-botkit-minibot:v1.0.0

# Update Kubernetes manifests
sed -i 's|ghcr.io/ag-botkit/monitor:.*|ghcr.io/your-org/ag-botkit-monitor:v1.0.0|g' deploy/k8s/monitor-deployment.yaml
sed -i 's|ghcr.io/ag-botkit/minibot:.*|ghcr.io/your-org/ag-botkit-minibot:v1.0.0|g' deploy/k8s/minibot-deployment.yaml

# Apply updates
kubectl apply -f deploy/k8s/monitor-deployment.yaml
kubectl apply -f deploy/k8s/minibot-deployment.yaml
```

---

## Monitoring & Observability

### Accessing Monitoring Tools

**Prometheus:**
```bash
kubectl port-forward -n ag-botkit svc/prometheus 9090:9090
# Open http://localhost:9090
```

**Grafana:**
```bash
kubectl port-forward -n ag-botkit svc/grafana 3000:3000
# Open http://localhost:3000
# Login: admin / <GRAFANA_PASSWORD>
```

**Monitor Dashboard:**
```bash
kubectl port-forward -n ag-botkit svc/monitor 8080:8080
# Open http://localhost:8080
```

### Key Metrics to Monitor

**System Health:**
- CPU usage (target: < 70%)
- Memory usage (target: < 80%)
- Disk usage (target: < 80%)
- Pod restarts (target: 0)

**Application Metrics:**
- RTDS lag (target: < 100ms)
- Message rate (baseline: varies)
- Risk decisions (rejection rate)
- Kill switch status

**Database:**
- Connection pool usage
- Query latency
- Storage usage
- Replication lag (if multi-AZ)

### Alert Configuration

Alerts are defined in `/infra/monitoring/alerts/rules.yaml` and include:
- High CPU/memory usage
- Pod crash looping
- Service unavailability
- RTDS connection loss
- Database issues
- Kill switch triggered

---

## Security

### Best Practices

**1. Secrets Management:**
- Never commit secrets to git
- Use Kubernetes Secrets or AWS Secrets Manager
- Rotate credentials regularly (90 days)
- Use strong passwords (32+ characters)

**2. Network Security:**
- Enable network policies
- Restrict ingress to specific IPs
- Use TLS for all external communication
- Isolate database in private subnet

**3. Image Security:**
- Run containers as non-root user
- Scan images for vulnerabilities
- Use minimal base images (Alpine)
- Pin image versions

**4. Access Control:**
- Use RBAC for Kubernetes access
- Implement least privilege IAM policies
- Enable audit logging
- Regular access reviews

### Security Scanning

```bash
# Scan Docker images
trivy image ghcr.io/ag-botkit/monitor:latest

# Scan Kubernetes manifests
kubesec scan deploy/k8s/monitor-deployment.yaml

# Audit Rust dependencies
cargo audit

# Check for secrets in code
git secrets --scan
```

---

## Troubleshooting

### Common Issues

**Problem: Pod won't start (ImagePullBackOff)**
```bash
# Check image name and registry
kubectl describe pod <pod-name> -n ag-botkit

# Verify registry credentials
kubectl get secret -n ag-botkit

# Solution: Verify image exists and credentials are correct
```

**Problem: Database connection failed**
```bash
# Check database pod
kubectl get pod timescaledb-0 -n ag-botkit
kubectl logs timescaledb-0 -n ag-botkit

# Test connectivity
kubectl exec -it deployment/monitor -n ag-botkit -- nc -zv timescaledb 5432

# Solution: Check secrets, network policies, and database status
```

**Problem: RTDS connection failures**
```bash
# Check minibot logs
kubectl logs -n ag-botkit deployment/minibot --tail=200

# Test external connectivity
kubectl exec -it deployment/minibot -n ag-botkit -- \
  wget -O- https://ws-live-data.polymarket.com 2>&1

# Solution: Check network policies, DNS, and firewall rules
```

**Problem: High latency**
```bash
# Check resource usage
kubectl top pods -n ag-botkit
kubectl top nodes

# Check HPA status
kubectl get hpa -n ag-botkit

# Solution: Scale up deployments or increase node resources
```

### Logs

```bash
# View pod logs
kubectl logs -n ag-botkit deployment/monitor --tail=100 -f

# View previous pod logs (if crashed)
kubectl logs -n ag-botkit <pod-name> --previous

# View logs from all pods in deployment
kubectl logs -n ag-botkit deployment/monitor --all-containers=true

# Save logs to file
kubectl logs -n ag-botkit deployment/monitor > monitor.log
```

### Debug Mode

```bash
# Start debug pod
kubectl run debug --image=busybox:latest -it --rm --restart=Never -n ag-botkit -- sh

# Inside debug pod:
# - Test DNS: nslookup monitor
# - Test connectivity: nc -zv monitor 8080
# - Test HTTP: wget -O- http://monitor:8080/health
```

---

## Performance Tuning

### Resource Limits

Default resource configuration (adjust based on load):

**Monitor:**
- Requests: 250m CPU, 256Mi RAM
- Limits: 1000m CPU, 1Gi RAM

**Minibot:**
- Requests: 500m CPU, 512Mi RAM
- Limits: 2000m CPU, 2Gi RAM

**TimescaleDB:**
- Requests: 1000m CPU, 2Gi RAM
- Limits: 4000m CPU, 8Gi RAM

### Horizontal Pod Autoscaling

```bash
# View HPA status
kubectl get hpa -n ag-botkit

# Manually scale deployment
kubectl scale deployment/monitor --replicas=5 -n ag-botkit

# Update HPA parameters
kubectl edit hpa monitor-hpa -n ag-botkit
```

### Database Optimization

```sql
-- Connect to database
kubectl exec -it timescaledb-0 -n ag-botkit -- psql -U agbot -d ag_botkit

-- Check slow queries
SELECT query, calls, mean_exec_time
FROM pg_stat_statements
ORDER BY mean_exec_time DESC
LIMIT 10;

-- Vacuum and analyze
VACUUM ANALYZE;

-- Check compression status
SELECT * FROM timescaledb_information.compressed_chunk_stats;
```

---

## Maintenance

### Regular Maintenance Tasks

**Daily:**
- Review dashboards for anomalies
- Check alert notifications
- Verify backup completion

**Weekly:**
- Review slow queries
- Check disk usage
- Update security patches

**Monthly:**
- Capacity planning review
- Test disaster recovery
- Audit access logs

### Backup and Restore

**Create Backup:**
```bash
# Database backup
kubectl exec timescaledb-0 -n ag-botkit -- \
  pg_dump -U agbot ag_botkit | gzip > backup-$(date +%Y%m%d).sql.gz

# Kubernetes resources
kubectl get all -n ag-botkit -o yaml > k8s-backup.yaml
```

**Restore Backup:**
```bash
# Restore database
gunzip -c backup-20250131.sql.gz | \
  kubectl exec -i timescaledb-0 -n ag-botkit -- \
  psql -U agbot -d ag_botkit
```

---

## Additional Resources

- [Deployment Runbook](../infra/ops/runbooks/deployment-runbook.md)
- [Disaster Recovery Plan](../infra/ops/runbooks/disaster-recovery.md)
- [MULTI_AGENT_PLAN.md](../MULTI_AGENT_PLAN.md) - System architecture
- [Kubernetes Documentation](https://kubernetes.io/docs/)
- [Terraform AWS Provider](https://registry.terraform.io/providers/hashicorp/aws/latest/docs)

---

## Support

**Issues:** https://github.com/your-org/ag-botkit/issues
**Slack:** #ag-botkit-ops
**Email:** devops@ag-botkit.example.com

---

**Last Updated:** 2025-12-31
**Maintained by:** DevOps Team
