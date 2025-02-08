import os
import pandas as pd

def write_to_excel(data, file_name, excel_dir):
    """Writes the running data into an Excel file per year and a sheet per month."""
    year, month = file_name.split("-")[:2]
    if not os.path.exists(excel_dir):
        os.makedirs(excel_dir)
    excel_path = os.path.join(excel_dir, f"{year}.xlsx")
    sheet_name = f"{month}"

    if os.path.exists(excel_path):
        with pd.ExcelWriter(excel_path, mode='a', if_sheet_exists='overlay') as writer:
            df_existing = pd.read_excel(writer,
                                        sheet_name=sheet_name) if sheet_name in writer.sheets else pd.DataFrame()
            df = pd.DataFrame([data])
            df = pd.concat([df_existing, df], ignore_index=True).drop_duplicates()
            df = df.sort_values(by=["Date", "Start Time (HH:mm)"], ascending=[True, True])
            df.to_excel(writer, sheet_name=sheet_name, index=False)
    else:
        df = pd.DataFrame([data])
        with pd.ExcelWriter(excel_path) as writer:
            df.to_excel(writer, sheet_name=sheet_name, index=False)
