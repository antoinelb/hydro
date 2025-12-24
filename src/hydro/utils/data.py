import polars as pl

#########
# types #
#########


NumericType = [
    # List of polars numeric data types for type filtering
    pl.Decimal,
    pl.Float32,
    pl.Float64,
    pl.Int8,
    pl.Int16,
    pl.Int32,
    pl.Int64,
    pl.Int128,
    pl.UInt8,
    pl.UInt16,
    pl.UInt32,
    pl.UInt64,
]
