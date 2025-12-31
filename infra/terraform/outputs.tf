# VPC Outputs
output "vpc_id" {
  description = "VPC ID"
  value       = module.vpc.vpc_id
}

output "vpc_cidr_block" {
  description = "VPC CIDR block"
  value       = module.vpc.vpc_cidr_block
}

output "private_subnets" {
  description = "List of private subnet IDs"
  value       = module.vpc.private_subnets
}

output "public_subnets" {
  description = "List of public subnet IDs"
  value       = module.vpc.public_subnets
}

output "database_subnets" {
  description = "List of database subnet IDs"
  value       = module.vpc.database_subnets
}

# EKS Outputs
output "eks_cluster_id" {
  description = "EKS cluster ID"
  value       = module.eks.cluster_id
}

output "eks_cluster_endpoint" {
  description = "EKS cluster endpoint"
  value       = module.eks.cluster_endpoint
}

output "eks_cluster_name" {
  description = "EKS cluster name"
  value       = module.eks.cluster_name
}

output "eks_cluster_certificate_authority_data" {
  description = "EKS cluster certificate authority data"
  value       = module.eks.cluster_certificate_authority_data
  sensitive   = true
}

output "eks_cluster_security_group_id" {
  description = "EKS cluster security group ID"
  value       = module.eks.cluster_security_group_id
}

output "eks_oidc_provider_arn" {
  description = "ARN of the OIDC Provider for EKS"
  value       = module.eks.oidc_provider_arn
}

# RDS Outputs
output "rds_instance_id" {
  description = "RDS instance ID"
  value       = module.rds.db_instance_identifier
}

output "rds_instance_endpoint" {
  description = "RDS instance endpoint"
  value       = module.rds.db_instance_endpoint
}

output "rds_instance_address" {
  description = "RDS instance address"
  value       = module.rds.db_instance_address
}

output "rds_instance_port" {
  description = "RDS instance port"
  value       = module.rds.db_instance_port
}

output "rds_instance_arn" {
  description = "RDS instance ARN"
  value       = module.rds.db_instance_arn
}

# Secrets Manager Outputs
output "rds_password_secret_arn" {
  description = "ARN of the RDS password secret in Secrets Manager"
  value       = aws_secretsmanager_secret.rds_password.arn
}

# kubectl configuration command
output "kubectl_config_command" {
  description = "Command to configure kubectl"
  value       = "aws eks update-kubeconfig --region ${var.aws_region} --name ${module.eks.cluster_name}"
}

# Database connection string (use with caution)
output "database_url" {
  description = "Database connection URL (sensitive)"
  value       = "postgresql://${var.db_username}:${random_password.rds_password.result}@${module.rds.db_instance_address}:${module.rds.db_instance_port}/${var.db_name}"
  sensitive   = true
}

# Kubeconfig for CI/CD
output "kubeconfig" {
  description = "Kubernetes config for CI/CD (sensitive)"
  value = {
    cluster_name                           = module.eks.cluster_name
    cluster_endpoint                       = module.eks.cluster_endpoint
    cluster_certificate_authority_data     = module.eks.cluster_certificate_authority_data
    region                                 = var.aws_region
  }
  sensitive = true
}
