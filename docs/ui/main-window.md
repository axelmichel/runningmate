# The interface

The main window of RunningMate is divided into three main sections: the menu bar, the activity list, and the activity details.

![Alt Text](../images/screens/main_01.png)
## Menu Bar (left)

Per default, you'll see all activities. You can easily filter the activities by type (running, cycling, walking) by clicking on the respective icon in the menu bar.
In the bottom part you'll have access to search, user - and system-settings.

## Activity list (center)
Above the list you'll find various key numbers, as the overall distance, duration and elevation gain. 
On the right, you see a activity heatmap.
Depending on calculated power of each activity the color becomes brighter. 
If you switch between the different activity types, the key numbers and the heatmap are updated with the stats of 
the respective activity type.

The activity list itself is sortable and has a pagination. Clicking on a row, the right side of the window will be updated, 
and you'll find the corresponding activity there. Clicking on the eye (deep purple button). You open the detail window. 
Clicking on the reloading button, the activity will be reprocessed. 
(In case some stats are not updating, or not updated, due to an older version of runningMate)
The delete button will delete the activity permanently from the db (and all corresponding files).

## Activity details (right)
In case no activity is selected, you'll see the last registered activity, 
in case you're using a filter for the type, the latest activity of the respective type.
When you click on a row in the activity list, you'll see the details of that activity. 
The box shows the following data:
- Date and Time
- Title
- Duration (HH:MM:SS), Pace (MM:SS)
- A track-map (if available)
- Distance (KM), Elevation Gain (M)
- Weather data if available: Overall condition, temperature and wind
