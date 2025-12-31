terraform {
  required_version = ">= 1.6"

  required_providers {
    aws = {
      source  = "hashicorp/aws"
      version = "~> 5.0"
    }
    kubernetes = {
      source  = "hashicorp/kubernetes"
      version = "~> 2.0"
    }
  }

  backend "s3" {
    bucket         = "ag-botkit-terraform-state"
    key            = "production/terraform.tfstate"
    region         = "us-east-1"
    encrypt        = true
    dynamodb_table = "ag-botkit-terraform-locks"
  }
}

provider "aws" {
  region = var.aws_region

  default_tags {
    tags = {
      Project     = "ag-botkit"
      Environment = var.environment
      ManagedBy   = "Terraform"
      Owner       = "devops-team"
    }
  }
}

# Data source for availability zones
data "aws_availability_zones" "available" {
  state = "available"
}

# Local variables
locals {
  name_prefix = "ag-botkit-${var.environment}"
  azs         = slice(data.aws_availability_zones.available.names, 0, 3)

  common_tags = {
    Project     = "ag-botkit"
    Environment = var.environment
    Terraform   = "true"
  }
}
