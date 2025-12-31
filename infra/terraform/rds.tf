# RDS TimescaleDB Configuration

# Generate random password for RDS
resource "random_password" "rds_password" {
  length  = 32
  special = true
}

# Store password in AWS Secrets Manager
resource "aws_secretsmanager_secret" "rds_password" {
  name                    = "${local.name_prefix}-rds-password"
  description             = "RDS master password for ag-botkit TimescaleDB"
  recovery_window_in_days = 7

  tags = local.common_tags
}

resource "aws_secretsmanager_secret_version" "rds_password" {
  secret_id = aws_secretsmanager_secret.rds_password.id
  secret_string = jsonencode({
    username = var.db_username
    password = random_password.rds_password.result
    engine   = "postgres"
    host     = module.rds.db_instance_address
    port     = module.rds.db_instance_port
    dbname   = var.db_name
  })
}

# RDS Subnet Group
resource "aws_db_subnet_group" "rds" {
  name       = "${local.name_prefix}-rds-subnet-group"
  subnet_ids = module.vpc.database_subnets

  tags = merge(
    local.common_tags,
    {
      Name = "${local.name_prefix}-rds-subnet-group"
    }
  )
}

# RDS Parameter Group for TimescaleDB optimization
resource "aws_db_parameter_group" "timescaledb" {
  name   = "${local.name_prefix}-timescaledb-pg15"
  family = "postgres15"

  # TimescaleDB optimizations
  parameter {
    name  = "shared_preload_libraries"
    value = "timescaledb"
  }

  parameter {
    name  = "max_connections"
    value = "500"
  }

  parameter {
    name  = "shared_buffers"
    value = "{DBInstanceClassMemory/4096}"  # 25% of RAM
  }

  parameter {
    name  = "effective_cache_size"
    value = "{DBInstanceClassMemory/2048}"  # 50% of RAM
  }

  parameter {
    name  = "maintenance_work_mem"
    value = "2097152"  # 2GB
  }

  parameter {
    name  = "checkpoint_completion_target"
    value = "0.9"
  }

  parameter {
    name  = "wal_buffers"
    value = "16384"  # 16MB
  }

  parameter {
    name  = "default_statistics_target"
    value = "100"
  }

  parameter {
    name  = "random_page_cost"
    value = "1.1"  # For SSD
  }

  parameter {
    name  = "effective_io_concurrency"
    value = "200"  # For SSD
  }

  parameter {
    name  = "work_mem"
    value = "10485"  # 10MB
  }

  parameter {
    name  = "min_wal_size"
    value = "2048"  # 2GB
  }

  parameter {
    name  = "max_wal_size"
    value = "8192"  # 8GB
  }

  # Logging
  parameter {
    name  = "log_min_duration_statement"
    value = "1000"  # Log queries > 1s
  }

  parameter {
    name  = "log_connections"
    value = "1"
  }

  parameter {
    name  = "log_disconnections"
    value = "1"
  }

  tags = local.common_tags
}

# RDS Module
module "rds" {
  source  = "terraform-aws-modules/rds/aws"
  version = "~> 6.0"

  identifier = "${local.name_prefix}-timescaledb"

  # Engine
  engine               = "postgres"
  engine_version       = "15.4"
  family               = "postgres15"
  major_engine_version = "15"
  instance_class       = var.rds_instance_class

  # Storage
  allocated_storage     = var.rds_allocated_storage
  max_allocated_storage = var.rds_max_allocated_storage
  storage_type          = "gp3"
  storage_encrypted     = true
  iops                  = 12000
  storage_throughput    = 500

  # Database
  db_name  = var.db_name
  username = var.db_username
  password = random_password.rds_password.result
  port     = 5432

  # Network
  db_subnet_group_name   = aws_db_subnet_group.rds.name
  vpc_security_group_ids = [aws_security_group.rds.id]
  publicly_accessible    = false

  # High Availability
  multi_az               = var.enable_rds_multi_az

  # Backup
  backup_retention_period = var.rds_backup_retention_period
  backup_window          = "03:00-04:00"
  maintenance_window     = "Mon:04:00-Mon:05:00"
  skip_final_snapshot    = var.environment == "staging" ? true : false
  final_snapshot_identifier_prefix = "${local.name_prefix}-final-snapshot"

  # Monitoring
  enabled_cloudwatch_logs_exports = ["postgresql", "upgrade"]
  monitoring_interval             = 60
  monitoring_role_name            = "${local.name_prefix}-rds-monitoring-role"
  create_monitoring_role          = true
  performance_insights_enabled    = true
  performance_insights_retention_period = 7

  # Parameter group
  parameter_group_name = aws_db_parameter_group.timescaledb.name

  # Deletion protection
  deletion_protection = var.environment == "production" ? true : false

  # Auto minor version upgrade
  auto_minor_version_upgrade = true

  tags = merge(
    local.common_tags,
    {
      Name = "${local.name_prefix}-timescaledb"
    }
  )
}

# CloudWatch alarms for RDS
resource "aws_cloudwatch_metric_alarm" "rds_cpu" {
  alarm_name          = "${local.name_prefix}-rds-cpu-utilization"
  comparison_operator = "GreaterThanThreshold"
  evaluation_periods  = "2"
  metric_name         = "CPUUtilization"
  namespace           = "AWS/RDS"
  period              = "300"
  statistic           = "Average"
  threshold           = "80"
  alarm_description   = "RDS CPU utilization is too high"
  alarm_actions       = []  # Add SNS topic ARN for notifications

  dimensions = {
    DBInstanceIdentifier = module.rds.db_instance_identifier
  }

  tags = local.common_tags
}

resource "aws_cloudwatch_metric_alarm" "rds_storage" {
  alarm_name          = "${local.name_prefix}-rds-free-storage"
  comparison_operator = "LessThanThreshold"
  evaluation_periods  = "1"
  metric_name         = "FreeStorageSpace"
  namespace           = "AWS/RDS"
  period              = "300"
  statistic           = "Average"
  threshold           = "10737418240"  # 10GB in bytes
  alarm_description   = "RDS free storage is too low"
  alarm_actions       = []  # Add SNS topic ARN for notifications

  dimensions = {
    DBInstanceIdentifier = module.rds.db_instance_identifier
  }

  tags = local.common_tags
}

resource "aws_cloudwatch_metric_alarm" "rds_connections" {
  alarm_name          = "${local.name_prefix}-rds-connections"
  comparison_operator = "GreaterThanThreshold"
  evaluation_periods  = "1"
  metric_name         = "DatabaseConnections"
  namespace           = "AWS/RDS"
  period              = "300"
  statistic           = "Average"
  threshold           = "450"  # 90% of max_connections
  alarm_description   = "RDS database connections are too high"
  alarm_actions       = []  # Add SNS topic ARN for notifications

  dimensions = {
    DBInstanceIdentifier = module.rds.db_instance_identifier
  }

  tags = local.common_tags
}
