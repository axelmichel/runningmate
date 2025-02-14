#Trainings Effect

Training Effect (TE) is a measure of how a workout impacts your fitness level. It helps you to understand whether your session improves endurance, speed, or strength or was a bit over the top. 

## Data Points
### Used data from the activity
- Heart Rate (HR) per second or per lap
- Cadence (if available)
- Speed/Distance (if available)
- Elevation (if available)

### Used data from the athlete
- Fixed Heart Rate Zones (see [Heart Rate Zones](heart-rate-zones.md))
  - or HRmax and HRrest
- Age (if available)
- Weight (if available)
- VO2max (if available)

## Calculation

1. **Heart Rate Zone Classification**  
     Determines how long you spend in different HR zones.

2. **EPOC Calculation**  
Determines how long you spend in different HR zones.
   > 
   > \(EPOC = \left( \frac{HR - HR_{baseline}}{HR_{max} - HR_{baseline}} \right) \times \text{duration} \times \text{factor}\)
    
    If the elevation is available, the EPOC is adjusted to the elevation factor. In general: **More elevation gain = higher EPOC impact.** The elevation factor is between **1.1x** for moderate climbs and **1.25x** for steep climbs.
   > 
   > \(EPOC = \text{intensity} \times \text{duration} \times 0.2 \times \text{elevation factor}\)
 
      If the weight is available, the EPOC is also adjusted to the athlete's weight. Because heavier athletes require more oxygen (VO₂) for the same intensity, leading to higher EPOC values.
   > 
   > \(EPOC_{\text{adjusted}} = EPOC_{\text{raw}} \times \left(\frac{\text{weight}}{70}\right)\)
   
3. **Training Effect Calculation**  
Uses a **logarithmic EPOC model**. Prevents **TE from exceeding 5.0**.
   > 
   > \(TE = 1.0 + \left( \frac{EPOC}{EPOC_{\text{threshold}}} \right)^{\text{scaling factor}}\)