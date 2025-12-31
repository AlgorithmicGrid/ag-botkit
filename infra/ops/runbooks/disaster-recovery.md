# Disaster Recovery Plan - ag-botkit

## Overview

This document outlines disaster recovery procedures for ag-botkit production infrastructure.

**Recovery Time Objective (RTO):** 4 hours
**Recovery Point Objective (RPO):** 1 hour

---

## Disaster Scenarios

### 1. Complete AWS Region Failure

**Severity:** Critical
**Impact:** Total service outage
**Recovery Time:** 3-4 hours

#### Response Procedure

1. **Declare Disaster**
   - Notify incident commander
   - Activate disaster recovery team
   - Update status page

2. **Failover to Backup Region**
   ```bash
   # Switch to DR region
   export AWS_REGION=us-west-2

   # Restore infrastructure from Terraform
   cd infra/terraform
   terraform workspace select dr
   terraform apply
   ```

3. **Restore Database from Snapshot**
   ```bash
   # Identify latest automated backup
   aws rds describe-db-snapshots \
     --db-instance-identifier ag-botkit-production-timescaledb \
     --region us-west-2

   # Restore from snapshot
   aws rds restore-db-instance-from-db-snapshot \
     --db-instance-identifier ag-botkit-dr-timescaledb \
     --db-snapshot-identifier <snapshot-id> \
     --region us-west-2
   ```

4. **Deploy Applications**
   ```bash
   # Update kubeconfig
   aws eks update-kubeconfig \
     --region us-west-2 \
     --name ag-botkit-dr-eks

   # Deploy applications
   kubectl apply -f deploy/k8s/
   ```

5. **Verify Functionality**
   - Test all health endpoints
   - Verify RTDS connectivity
   - Check data integrity
   - Monitor metrics for 30 minutes

6. **Update DNS**
   ```bash
   # Update Route53 records to point to DR region
   # This should be automated via health checks
   ```

---

### 2. Database Corruption/Data Loss

**Severity:** High
**Impact:** Data integrity compromised
**Recovery Time:** 1-2 hours

#### Response Procedure

1. **Assess Damage**
   ```bash
   # Connect to database
   kubectl exec -it timescaledb-0 -n ag-botkit -- \
     psql -U agbot -d ag_botkit

   # Check for corrupted tables
   SELECT * FROM timescaledb_information.hypertables;
   ```

2. **Activate Kill Switch**
   ```bash
   # Stop all trading immediately
   kubectl exec -it deployment/minibot -n ag-botkit -- \
     curl -X POST http://localhost:9091/api/kill-switch/trigger
   ```

3. **Restore from Backup**
   ```bash
   # Get latest backup
   aws s3 ls s3://ag-botkit-backups/database/ --recursive | tail -n 1

   # Download backup
   aws s3 cp s3://ag-botkit-backups/database/backup-YYYYMMDD.sql.gz .

   # Decompress
   gunzip backup-YYYYMMDD.sql.gz

   # Restore (this will drop existing database!)
   kubectl exec -i timescaledb-0 -n ag-botkit -- \
     psql -U agbot -d postgres -c "DROP DATABASE ag_botkit;"
   kubectl exec -i timescaledb-0 -n ag-botkit -- \
     psql -U agbot -d postgres -c "CREATE DATABASE ag_botkit;"
   kubectl exec -i timescaledb-0 -n ag-botkit -- \
     psql -U agbot -d ag_botkit < backup-YYYYMMDD.sql
   ```

4. **Verify Data Integrity**
   ```bash
   # Run integrity checks
   kubectl exec -it timescaledb-0 -n ag-botkit -- \
     psql -U agbot -d ag_botkit -c "SELECT count(*) FROM metrics;"

   # Compare with expected values
   ```

5. **Resume Operations**
   - Reset kill switch
   - Monitor for issues
   - Document incident

---

### 3. Kubernetes Cluster Failure

**Severity:** High
**Impact:** Service outage
**Recovery Time:** 2-3 hours

#### Response Procedure

1. **Diagnose Cluster State**
   ```bash
   # Check node status
   kubectl get nodes

   # Check control plane health
   kubectl get componentstatuses

   # Check etcd health
   kubectl get --raw='/healthz?verbose'
   ```

2. **Attempt Cluster Recovery**
   ```bash
   # Restart failed nodes
   aws ec2 reboot-instances \
     --instance-ids <instance-ids>

   # If nodes don't recover, recreate node group
   aws eks update-nodegroup-version \
     --cluster-name ag-botkit-production-eks \
     --nodegroup-name general \
     --force
   ```

3. **If Recovery Fails - Rebuild Cluster**
   ```bash
   # Backup current state
   kubectl get all -n ag-botkit -o yaml > backup-state.yaml

   # Destroy and recreate via Terraform
   cd infra/terraform
   terraform destroy -target=module.eks
   terraform apply -target=module.eks

   # Redeploy applications
   kubectl apply -f deploy/k8s/
   ```

---

### 4. Security Breach

**Severity:** Critical
**Impact:** Data confidentiality/integrity at risk
**Recovery Time:** Variable

#### Response Procedure

1. **Immediate Actions**
   - Isolate affected systems
   - Activate kill switch
   - Preserve forensic evidence
   - Notify security team

2. **Contain Breach**
   ```bash
   # Rotate all credentials immediately
   kubectl delete secret --all -n ag-botkit

   # Recreate secrets with new values
   # (Follow secrets creation procedure)

   # Update API keys with exchanges
   # (Manual process with each exchange)
   ```

3. **Investigate**
   - Review CloudTrail logs
   - Check application logs
   - Analyze network traffic
   - Identify entry point

4. **Remediate**
   - Patch vulnerabilities
   - Update security groups
   - Rotate all keys and tokens
   - Rebuild compromised systems

5. **Validate Security**
   - Run security audit
   - Penetration testing
   - Review IAM policies
   - Update security procedures

---

## Backup Procedures

### Automated Backups

**RDS Automated Backups:**
- Frequency: Daily at 03:00 UTC
- Retention: 30 days
- Location: Same region + cross-region replica
- Type: Full snapshot

**Application State:**
- ConfigMaps/Secrets backed up to S3 (encrypted)
- Frequency: On every change
- Retention: 90 days

### Manual Backup Creation

```bash
# Database backup
kubectl exec -it timescaledb-0 -n ag-botkit -- \
  pg_dump -U agbot ag_botkit | gzip > backup-$(date +%Y%m%d-%H%M%S).sql.gz

# Upload to S3
aws s3 cp backup-*.sql.gz s3://ag-botkit-backups/database/

# Kubernetes resources backup
kubectl get all -n ag-botkit -o yaml > k8s-backup-$(date +%Y%m%d).yaml
aws s3 cp k8s-backup-*.yaml s3://ag-botkit-backups/kubernetes/

# Terraform state backup
cd infra/terraform
terraform state pull > terraform-state-$(date +%Y%m%d).json
aws s3 cp terraform-state-*.json s3://ag-botkit-backups/terraform/
```

### Backup Verification

**Monthly Backup Restoration Test:**
1. Create test environment
2. Restore latest backup
3. Verify data integrity
4. Test application functionality
5. Document results
6. Destroy test environment

---

## Recovery Testing Schedule

**Quarterly:**
- Full disaster recovery drill (complete region failover)
- Database recovery test
- Document lessons learned

**Monthly:**
- Backup restoration verification
- Update recovery procedures
- Review and test runbooks

**Weekly:**
- Verify backup completion
- Test access to backup storage
- Review monitoring alerts

---

## Communication Plan

### Incident Severity Levels

**P0 - Critical:**
- Complete service outage
- Data loss/corruption
- Security breach
- Notify: All stakeholders within 15 minutes

**P1 - High:**
- Partial service degradation
- Database issues (not critical)
- Notify: Technical team within 30 minutes

**P2 - Medium:**
- Individual component failure with redundancy
- Notify: Technical team within 2 hours

### Communication Channels

**Internal:**
- Slack: #ag-botkit-incidents
- PagerDuty: On-call engineer
- Email: incidents@ag-botkit.example.com

**External:**
- Status page: status.ag-botkit.example.com
- Email: users@ag-botkit.example.com
- Twitter: @agbotkit_status

### Incident Commander Responsibilities

1. Coordinate recovery efforts
2. Maintain communication log
3. Make critical decisions
4. Update stakeholders
5. Declare incident resolved
6. Lead post-mortem

---

## Post-Incident Procedures

### Immediate (Within 24 hours)

1. **Document Timeline**
   - When did incident start?
   - Detection time
   - Response timeline
   - Resolution time

2. **Preserve Evidence**
   - Logs
   - Metrics
   - Screenshots
   - Configuration states

3. **Update Status**
   - Internal notification
   - External status page
   - Customer communication

### Short-term (Within 1 week)

1. **Conduct Post-Mortem**
   - What happened?
   - Why did it happen?
   - How was it resolved?
   - What can we improve?

2. **Create Action Items**
   - Preventive measures
   - Process improvements
   - Documentation updates
   - Training needs

3. **Update Documentation**
   - Runbooks
   - Architecture diagrams
   - Recovery procedures

### Long-term (Within 1 month)

1. **Implement Improvements**
   - Code changes
   - Infrastructure updates
   - Process changes
   - Additional monitoring

2. **Share Learnings**
   - Team presentation
   - Written summary
   - Update training materials
   - Industry sharing (if appropriate)

---

## Emergency Contacts

**Incident Commander:** [Name] - [Phone] - [Email]
**AWS Support:** [Account Manager] - [Support Phone]
**Database Expert:** [Name] - [Phone] - [Email]
**Security Lead:** [Name] - [Phone] - [Email]
**Executive Sponsor:** [Name] - [Phone] - [Email]

**External Services:**
- AWS Support: Enterprise Support Case
- Exchange Support: [Contact info for each exchange]

---

## Critical Credentials

**Location:** AWS Secrets Manager
- Secret Name: `ag-botkit/production/dr-credentials`
- Region: us-east-1 (primary), us-west-2 (replica)
- Access: DR team IAM role only

**Break-Glass Access:**
- Root AWS account credentials: Physical safe
- Database master password: Password manager (CTO access)
- Encryption keys: AWS KMS + offline backup

---

**Last Updated:** 2025-12-31
**Document Owner:** DevOps Team
**Review Frequency:** Quarterly
**Next Review:** 2026-03-31
