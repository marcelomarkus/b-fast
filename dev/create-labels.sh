#!/bin/bash
# Script to create GitHub labels for B-FAST repository
# Usage: ./create-labels.sh

REPO="marcelomarkus/b-fast"

echo "🏷️  Creating labels for $REPO..."

# Automatic labels (added by CI)
gh label create "feat" --color "0E8A16" --description "Feature branch" --force
gh label create "fix" --color "D73A4A" --description "Bug fix branch" --force
gh label create "docs" --color "0075CA" --description "Documentation branch" --force

# Approval labels
gh label create "approved-1" --color "FBCA04" --description "1 approval" --force
gh label create "approved-2" --color "FFA500" --description "2 approvals" --force
gh label create "approved-3" --color "FF8C00" --description "3+ approvals" --force

# Manual labels
gh label create "bug" --color "D73A4A" --description "Something isn't working" --force
gh label create "enhancement" --color "A2EEEF" --description "New feature or improvement" --force
gh label create "documentation" --color "0075CA" --description "Documentation improvements" --force
gh label create "good first issue" --color "7057FF" --description "Good for newcomers" --force
gh label create "help wanted" --color "008672" --description "Extra attention needed" --force
gh label create "python" --color "3776AB" --description "Python-specific issues" --force
gh label create "typescript" --color "3178C6" --description "TypeScript client issues" --force
gh label create "rust" --color "CE422B" --description "Rust core library issues" --force
gh label create "performance" --color "FBCA04" --description "Performance-related changes" --force

echo "✅ Labels created successfully!"
echo ""
echo "📝 Note: You need GitHub CLI (gh) installed and authenticated."
echo "   Install: https://cli.github.com/"
echo "   Auth: gh auth login"
