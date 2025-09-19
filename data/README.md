# Data Directory

Place your market data export files (comma-separated text) in this folder. They are ignored by git.

Required minimum columns (comma separated):
- Date, Time, Open, High, Low, Close, Volume
OR
- Datetime, Open, High, Low, Close, Volume

Examples:
```
2025-09-19,09:30:00,4500.25,4502.00,4498.50,4501.75,1234
2025-09-19 09:31:00,4501.75,4503.00,4499.25,4500.50,987
```

Large or sensitive data will not be committed due to `.gitignore` rules.
